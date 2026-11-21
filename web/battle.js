import { readAxes, revealCell } from "./board.js";

const names = [
  "ミジンコ", "ありんこ", "ネズミ", "うさぎ", "あかちゃん", "おんなのこ",
  "なだこうせい", "とうだいせい", "ヒュパティア", "ピタゴラス", "フェルマー", "ラマヌジャン",
];
const symbols = ["+", "−", "×"];

export function setupBattle(wasm, grantReward) {
  const grid = document.querySelector("#battle-grid");
  const roster = document.querySelector("#roster");
  const portrait = document.querySelector("#opponent-portrait");
  const opponentName = document.querySelector("#opponent-name");
  const operation = document.querySelector("#battle-operation");
  const digitsPicker = document.querySelector("#battle-digits");
  const start = document.querySelector("#battle-start");
  const question = document.querySelector("#question");
  const form = document.querySelector("#answer-form");
  const input = document.querySelector("#battle-answer");
  const note = document.querySelector("#battle-note");
  const playerProgress = document.querySelector("#player-progress");
  const opponentProgress = document.querySelector("#opponent-progress");
  const playerText = document.querySelector("#player-progress-text");
  const opponentText = document.querySelector("#opponent-progress-text");
  const progress = document.querySelector("#board-progress");
  const raceTime = document.querySelector("#race-time");
  const expression = document.querySelector("#expression");
  const cutin = document.querySelector("#race-cutin");
  const cutinPortrait = document.querySelector("#cutin-portrait");
  const cutinTitle = document.querySelector("#cutin-title");
  const cutinDetail = document.querySelector("#cutin-detail");
  const cutinClose = document.querySelector("#cutin-close");

  let opponent = 0;
  let rows, cols, roundOperation, squares, started, timer, seed;
  let highlighted, rowHeaders, colHeaders;
  let cutinTimer, afterCutin, reactionUntil, opponentLines;
  const elapsed = () => Math.floor(performance.now() - started);
  const value = field => wasm.battle_value(field);
  const formatTime = ms => {
    const seconds = Math.floor(ms / 1000);
    return [Math.floor(seconds / 60), seconds % 60].map(part => String(part).padStart(2, "0")).join(":");
  };
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

  function setPortrait(element, mood = "") {
    element.style.setProperty("--col", opponent % 4);
    element.style.setProperty("--row", Math.floor(opponent / 4));
    element.dataset.mood = mood;
  }

  function selectOpponent() {
    [...roster.children].forEach((button, index) => {
      button.setAttribute("aria-pressed", index === opponent);
    });
    setPortrait(portrait);
    opponentName.textContent = names[opponent];
  }

  function react(mood, text, duration = 2200) {
    setPortrait(portrait, mood);
    expression.textContent = text;
    reactionUntil = performance.now() + duration;
  }

  function clearCutin() {
    clearTimeout(cutinTimer);
    afterCutin = undefined;
    if (cutin.open) cutin.close();
  }

  function finishCutin() {
    const next = afterCutin;
    clearCutin();
    next?.();
  }

  function showCutin(title, detail, mood, duration, next) {
    clearCutin();
    setPortrait(cutinPortrait, mood);
    cutinTitle.textContent = title;
    cutinDetail.textContent = detail;
    afterCutin = next;
    cutin.showModal();
    cutinTimer = setTimeout(finishCutin, duration);
  }

  cutinClose.addEventListener("click", finishCutin);
  cutin.addEventListener("cancel", event => {
    event.preventDefault();
    finishCutin();
  });

  const randomSeed = () => crypto.getRandomValues(new Uint32Array(1))[0];

  function drawGrid(seed) {
    const digits = Number(digitsPicker.value);
    [rows, cols] = readAxes(wasm, seed, digits);
    grid.dataset.digits = digits;
    grid.closest(".board").scrollLeft = 0;
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
        pick.setAttribute("aria-label", `${r + 1}ぎょう ${c + 1}れつ、${row} ${symbols[roundOperation]} ${col}`);
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
    if (current >= 0) revealCell(squares[current]);
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
    const next = value(2);
    highlight(next);
    input.value = "";
    if (event.currentTarget === grid) squares[next].firstElementChild.focus();
  };
  grid.addEventListener("keydown", moveCell);
  input.addEventListener("keydown", moveCell);

  function setProgress(player, cpu) {
    playerProgress.value = player;
    opponentProgress.value = cpu;
    playerText.textContent = `${player} / 100`;
    opponentText.textContent = `${cpu} / 100`;
    progress.textContent = `${player} / 100 マス`;
  }

  function refresh() {
    const now = elapsed();
    wasm.battle_tick(now);
    const player = value(0);
    const cpu = value(1);
    setProgress(player, cpu);
    raceTime.textContent = formatTime(now);
    const status = value(3);
    if (status === 0) {
      highlight(value(2));
      if (performance.now() >= reactionUntil) react("", "", 0);
      const lines = Math.floor(cpu / 10);
      if (lines > opponentLines) {
        react("laugh", "あいてのラインアタック！");
        opponentLines = lines;
      }
      return status;
    }

    clearInterval(timer);
    enableAnswer(false);
    highlight(-1);
    for (const square of squares) square.firstElementChild.disabled = true;
    const won = status === 1;
    const title = won ? "100マス できた！" : `${names[opponent]}のかち！`;
    const detail = `あなた ${player} マス ／ ${names[opponent]} ${cpu} マス`;
    const mood = won ? "worried" : "laugh";
    react(mood, "レースおわり");
    note.textContent = won ? "かち！ 100マスを うめたよ。" : "あいてが さきに 100マスを うめたよ。";
    start.textContent = "もういちど";
    showCutin(title, detail, mood, 2600, () => start.focus());
    if (won) {
      const roundStarted = started;
      grantReward(wasm.battle_reward()).then(reward => {
        if (reward && started === roundStarted) {
          note.textContent += ` ${reward}`;
          if (cutin.open) cutinDetail.textContent = `${detail} · ${reward}`;
        }
      });
    }
    return status;
  }

  function resetBattle() {
    clearInterval(timer);
    clearCutin();
    enableAnswer(false);
    input.value = "";
    started = undefined;
    seed = randomSeed();
    roundOperation = Number(operation.value);
    drawGrid(seed);
    question.textContent = "スタートを おしてね";
    note.textContent = "マスを えらんで、あいてより さきに 100マスを うめよう。";
    raceTime.textContent = "00:00";
    setProgress(0, 0);
    react("", "", 0);
    start.textContent = "スタート";
  }

  start.addEventListener("click", () => {
    clearInterval(timer);
    enableAnswer(false);
    if (started !== undefined) seed = randomSeed();
    roundOperation = Number(operation.value);
    wasm.battle_start(seed, roundOperation, opponent, Number(digitsPicker.value));
    drawGrid(seed);
    setProgress(0, 0);
    raceTime.textContent = "00:00";
    opponentLines = 0;
    react("", "", 0);
    question.textContent = "まもなくスタート";
    note.textContent = "";
    input.value = "";
    showCutin("100マスレース！", `あいては ${names[opponent]}。よーい、スタート！`, "", 1500, () => {
      started = performance.now();
      enableAnswer(true);
      for (const square of squares) square.firstElementChild.disabled = false;
      input.inputMode = roundOperation === 1 ? "text" : "numeric";
      note.textContent = "クリック・タップ・やじるしキーで マスを えらべるよ。";
      start.textContent = "やりなおす";
      refresh();
      timer = setInterval(refresh, 100);
      input.focus();
    });
  });

  form.addEventListener("submit", event => {
    event.preventDefault();
    if (input.disabled) return;
    const answer = input.value.trim().normalize("NFKC").replace(/−/g, "-");
    if (!/^-?\d{1,6}$/.test(answer)) {
      note.textContent = "かずを いれてね。";
      input.focus();
      return;
    }
    const now = elapsed();
    const cell = value(2);
    const mark = wasm.battle_submit(Number(answer), now);
    if (mark === 2) {
      const square = squares[cell];
      const button = square.firstElementChild;
      button.textContent = Number(answer);
      button.setAttribute("aria-label", `${button.getAttribute("aria-label")} = ${Number(answer)}、せいかいずみ`);
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
      const lines = value(4);
      note.textContent = lines ? `${lines}ラインできた！ ラインアタック！` : "せいかい！";
      if (lines) react("worried", "ラインアタック！");

      input.value = "";
    } else if (mark === 1) {
      note.textContent = "おしい！ おなじマスに もういちど こたえよう。";
      input.select();
    }
    if (refresh() === 0) input.focus();
  });

  operation.addEventListener("change", resetBattle);
  digitsPicker.addEventListener("change", resetBattle);
  selectOpponent();
  resetBattle();
  return resetBattle;
}
