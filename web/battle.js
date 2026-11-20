const names = [
  "ミジンコ", "ありんこ", "ネズミ", "うさぎ", "赤ちゃん", "女の子",
  "灘高生", "東大生", "ヒュパティア", "ピタゴラス", "フェルマー", "ラマヌジャン",
];
const symbols = ["+", "−", "×"];

export function setupBattle(wasm) {
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
  const countdown = document.querySelector("#countdown");

  let opponent = 0;
  let rows, cols, roundOperation, squares, started, timer, seed;
  let attacks = 0;
  let lastCell = 0;
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
    head.insertRow().append(node("th", symbols[roundOperation]),
      ...cols.map(col => Object.assign(node("th", col), { scope: "col" })));
    const body = document.createElement("tbody");
    rows.forEach(row => {
      const line = body.insertRow();
      line.append(Object.assign(node("th", row), { scope: "row" }));
      cols.forEach(() => squares.push(line.insertCell()));
    });
    grid.replaceChildren(head, body);
  }

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
    squares[lastCell].classList.remove("current");
    if (status === 0) {
      squares[current].classList.add("current");
      question.textContent = `${rows[Math.floor(current / 10)]} ${symbols[roundOperation]} ${cols[current % 10]} = ?`;
      countdown.textContent = `相手の攻撃まで ${Math.ceil(value(7) / 1000)} 秒`;
    } else {
      clearInterval(timer);
      enableAnswer(false);
      countdown.textContent = "対戦終了";
      note.textContent = status === 1 ? "勝ち！ よくできました。" : "相手の勝ち。もう一度チャレンジ！";
      start.textContent = "もう一度";
      start.focus();
    }
    lastCell = current;
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
    note.textContent = "正解が続くほど攻撃が強くなります。";
    countdown.textContent = "";
    playerHp.value = 36;
    document.querySelector("#player-hp-text").textContent = "36 / 36";
    opponentHp.max = 22 + 3 * opponent;
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
    lastCell = 0;
    attacks = 0;
    started = performance.now();
    enableAnswer(true);
    input.inputMode = roundOperation === 1 ? "text" : "numeric";
    input.value = "";
    note.textContent = "答えを入力して攻撃しよう！";
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
      squares[cell].textContent = answer;
      squares[cell].classList.add("solved");
      note.textContent = `正解！ ${wasm.battle_value(3, now)}連鎖で攻撃！`;
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
