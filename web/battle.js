const names = [
  "ミジンコ", "ありんこ", "ネズミ", "うさぎ", "赤ちゃん", "女の子",
  "灘高生", "東大生", "ヒュパティア", "ピタゴラス", "フェルマー", "ラマヌジャン",
];
const symbols = ["+", "−", "×"];

export function setupBattle(wasm, grantReward) {
  const grid = document.querySelector("#battle-grid");
  const roster = document.querySelector("#roster");
  const portrait = document.querySelector("#opponent-portrait");
  const opponentName = document.querySelector("#opponent-name");
  const operation = document.querySelector("#battle-operation");
  const start = document.querySelector("#battle-start");
  const question = document.querySelector("#question");
  const form = document.querySelector("#answer-form");
  const input = document.querySelector("#battle-answer");
  const note = document.querySelector("#battle-note");
  const playerHp = document.querySelector("#player-hp");
  const opponentHp = document.querySelector("#opponent-hp");
  const combo = document.querySelector("#combo");
  const progress = document.querySelector("#board-progress");
  const countdown = document.querySelector("#countdown");

  let opponent = 0;
  let rows, cols, roundOperation, squares, started, timer, seed;
  let attacks = 0;
  let highlighted;
  let rowHeaders, colHeaders;
  const elapsed = () => Math.floor(performance.now() - started);
  const enableAnswer = enabled => {
    input.disabled = !enabled;
    form.querySelector("button").disabled = !enabled;
  };

  names.forEach((name, index) => {
    const button = document.createElement("button");
    const art = document.createElement("span");
    art.className = "portrait";
    art.style.setProperty("--col", index % 4);
    art.style.setProperty("--row", Math.floor(index / 4));
    button.type = "button";
    button.append(art, document.createTextNode(name));
    button.addEventListener("click", () => {
      opponent = index;
      selectOpponent();
      resetBattle();
    });
    roster.append(button);
  });

  function selectOpponent() {
    [...roster.children].forEach((button, index) => {
      button.setAttribute("aria-pressed", index === opponent);
    });
    portrait.style.setProperty("--col", opponent % 4);
    portrait.style.setProperty("--row", Math.floor(opponent / 4));
    opponentName.textContent = names[opponent];
  }

  const randomSeed = () => crypto.getRandomValues(new Uint32Array(1))[0];

  function drawGrid(seed) {
    const axis = index => String(wasm.make_axis(seed, index)).padStart(10, "0").split("").map(Number);
    rows = axis(0);
    cols = axis(1);
    squares = [];
    const node = (tag, text) => Object.assign(document.createElement(tag), { textContent: text });
    const head = document.createElement("thead");
    colHeaders = cols.map(col => Object.assign(node("th", col), { scope: "col" }));
    head.insertRow().append(node("th", symbols[roundOperation]), ...colHeaders);
    const body = document.createElement("tbody");
    rowHeaders = rows.map(row => Object.assign(node("th", row), { scope: "row" }));
    rows.forEach((row, r) => {
      const line = body.insertRow();
      line.append(rowHeaders[r]);
      cols.forEach((col, c) => {
        const square = line.insertCell();
        const pick = document.createElement("button");
        const index = r * 10 + c;
        pick.type = "button";
        pick.tabIndex = -1;
        pick.disabled = true;
        pick.dataset.cell = index;
        pick.setAttribute("aria-label", `${r + 1}行 ${c + 1}列、${row} ${symbols[roundOperation]} ${col}`);
        square.append(pick);
        squares.push(square);
      });
    });
    highlighted = undefined;
    grid.replaceChildren(head, body);
  }

  function highlight(current) {
    if (highlighted === current) return;
    highlighted = current;
    const row = Math.floor(current / 10);
    const col = current % 10;
    squares.forEach((square, index) => {
      square.classList.toggle("current", index === current);
      square.classList.toggle("crosshair", row === Math.floor(index / 10) || col === index % 10);
      square.firstElementChild.tabIndex = index === current ? 0 : -1;
      square.firstElementChild.setAttribute("aria-pressed", index === current);
    });
    rowHeaders.forEach((header, index) => header.classList.toggle("axis-active", index === row));
    colHeaders.forEach((header, index) => header.classList.toggle("axis-active", index === col));
    if (current >= 0) question.textContent = `${rows[row]} ${symbols[roundOperation]} ${cols[col]} = ?`;
  }

  function selectCell(index) {
    if (input.disabled || !wasm.battle_select(index)) return;
    highlight(index);
    input.value = "";
    input.focus();
  }

  grid.addEventListener("click", event => {
    const button = event.target.closest("[data-cell]");
    if (button) selectCell(Number(button.dataset.cell));
  });

  const directions = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
  const moveCell = event => {
    if (event.isComposing || event.altKey || event.ctrlKey || event.metaKey || input.disabled) return;
    if (!Object.hasOwn(directions, event.key)) return;
    event.preventDefault();
    if (!wasm.battle_move(...directions[event.key])) return;
    const next = wasm.battle_value(4, elapsed());
    highlight(next);
    input.value = "";
    if (event.currentTarget === grid) squares[next].firstElementChild.focus();
  };
  grid.addEventListener("keydown", moveCell);
  input.addEventListener("keydown", moveCell);

  function refresh() {
    const now = elapsed();
    wasm.battle_tick(now);
    const value = field => wasm.battle_value(field, now);
    playerHp.value = value(0);
    document.querySelector("#player-hp-text").textContent = `${value(0)} / 36`;
    opponentHp.max = value(2);
    opponentHp.value = value(1);
    document.querySelector("#opponent-hp-text").textContent = `${value(1)} / ${value(2)}`;
    combo.textContent = value(3);
    const current = value(4);
    const status = value(6);
    progress.textContent = `${value(8)} / 100 マス`;
    if (status === 0) {
      highlight(current);
      countdown.textContent = `相手の攻撃まで ${Math.ceil(value(7) / 1000)} 秒`;
    } else {
      clearInterval(timer);
      enableAnswer(false);
      highlight(-1);
      for (const square of squares) square.firstElementChild.disabled = true;
      countdown.textContent = "対戦終了";
      note.textContent = status === 1 ? (value(9) ? "ライン完成で勝ち！" : "勝ち！ よくできました。") : "相手の勝ち。もう一度チャレンジ！";
      start.textContent = "もう一度";
      start.focus();
      if (status === 1) {
        const roundStarted = started;
        grantReward(wasm.battle_reward()).then(reward => {
          if (reward && started === roundStarted) note.textContent += ` ${reward}`;
        });
      }
    }
    if (value(5) !== attacks && status === 0) note.textContent = `${names[opponent]}の攻撃！`;
    attacks = value(5);
    return status;
  }

  function resetBattle() {
    clearInterval(timer);
    enableAnswer(false);
    input.value = "";
    started = undefined;
    seed = randomSeed();
    roundOperation = Number(operation.value);
    drawGrid(seed);
    question.textContent = "対戦開始を押してね";
    note.textContent = "マスを選んで、たて・よこのラインを完成させよう。";
    progress.textContent = "0 / 100 マス";
    countdown.textContent = "";
    playerHp.value = 36;
    document.querySelector("#player-hp-text").textContent = "36 / 36";
    opponentHp.max = wasm.battle_health(opponent);
    opponentHp.value = opponentHp.max;
    document.querySelector("#opponent-hp-text").textContent = `${opponentHp.max} / ${opponentHp.max}`;
    combo.textContent = 0;
    start.textContent = "対戦開始";
  }

  start.addEventListener("click", () => {
    clearInterval(timer);
    if (started !== undefined) seed = randomSeed();
    roundOperation = Number(operation.value);
    wasm.battle_start(seed, roundOperation, opponent);
    drawGrid(seed);
    attacks = 0;
    started = performance.now();
    enableAnswer(true);
    for (const square of squares) square.firstElementChild.disabled = false;
    input.inputMode = roundOperation === 1 ? "text" : "numeric";
    input.value = "";
    note.textContent = "マスをクリック・タップ、または矢印キーで選べます。";
    start.textContent = "対戦をやり直す";
    refresh();
    timer = setInterval(refresh, 100);
    input.focus();
  });

  form.addEventListener("submit", event => {
    event.preventDefault();
    if (input.disabled) return;
    const answer = input.value.trim().normalize("NFKC").replace(/−/g, "-");
    if (!/^-?\d{1,3}$/.test(answer)) {
      note.textContent = "整数を入力してね。";
      input.focus();
      return;
    }
    const now = elapsed();
    const cell = wasm.battle_value(4, now);
    const mark = wasm.battle_submit(Number(answer), now);
    if (mark === 2) {
      const square = squares[cell];
      const button = square.firstElementChild;
      button.textContent = Number(answer);
      button.setAttribute("aria-label", `${button.getAttribute("aria-label")} = ${Number(answer)}、正解済み`);
      button.disabled = true;
      square.classList.add("solved");
      const row = Math.floor(cell / 10);
      const col = cell % 10;
      if (squares.slice(row * 10, row * 10 + 10).every(item => item.classList.contains("solved"))) {
        rowHeaders[row].classList.add("line-complete");
      }
      if (squares.filter((_, index) => index % 10 === col).every(item => item.classList.contains("solved"))) {
        colHeaders[col].classList.add("line-complete");
      }
      const lines = wasm.battle_value(9, now);
      note.textContent = lines ? `${lines}ライン完成！ ボーナス攻撃！` : `正解！ ${wasm.battle_value(3, now)}連鎖で攻撃！`;
      input.value = "";
    } else if (mark === 1) {
      note.textContent = "おしい！ 同じマスにもう一度答えよう。";
      input.select();
    }
    if (refresh() === 0) input.focus();
  });

  operation.addEventListener("change", resetBattle);
  selectOpponent();
  resetBattle();
  return resetBattle;
}
