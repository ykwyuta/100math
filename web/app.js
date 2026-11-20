import { setupBattle } from "./battle.js";

const side = 10;
const symbols = ["+", "−", "×"];
const table = document.querySelector("#grid");
const operations = document.querySelector(".operations");
const newButton = document.querySelector("#new");
const checkButton = document.querySelector("#check");
const count = document.querySelector("#count");
const time = document.querySelector("#time");
const result = document.querySelector("#result");

let wasm;
let operation = 0;
let rows;
let columns;
let cells = [];
let started = null;
let timer;
let graded = false;
let previousSeed;

function updateTime() {
  const seconds = started === null ? 0 : Math.floor((performance.now() - started) / 1000);
  time.textContent = [Math.floor(seconds / 60), seconds % 60]
    .map(value => String(value).padStart(2, "0")).join(":");
}

function cell(tag, value, scope) {
  const element = document.createElement(tag);
  element.textContent = value;
  if (scope) element.scope = scope;
  return element;
}

function newBoard(focus) {
  clearInterval(timer);
  started = null;
  graded = false;
  updateTime();
  count.textContent = "0";
  result.textContent = "答えを入力すると、計測が始まります。";
  checkButton.disabled = true;

  const random = new Uint32Array(1);
  do {
    crypto.getRandomValues(random);
  } while (random[0] === previousSeed);
  previousSeed = random[0];
  const axis = index => String(wasm.make_axis(random[0], index)).padStart(side, "0").split("").map(Number);
  rows = axis(0);
  columns = axis(1);

  const head = document.createElement("thead");
  const headerRow = head.insertRow();
  const corner = cell("th", symbols[operation]);
  corner.className = "corner";
  headerRow.append(corner, ...columns.map(value => cell("th", value, "col")));

  const body = document.createElement("tbody");
  cells = [];
  rows.forEach((left, row) => {
    const line = body.insertRow();
    line.append(cell("th", left, "row"));
    columns.forEach((top, column) => {
      const input = document.createElement("input");
      input.type = "text";
      input.inputMode = operation === 1 ? "text" : "numeric";
      input.autocomplete = "off";
      input.spellcheck = false;
      input.setAttribute("aria-label", `${row + 1}行 ${column + 1}列：${left} ${symbols[operation]} ${top}`);
      line.insertCell().append(input);
      cells.push(input);
    });
  });
  table.replaceChildren(head, body);
  if (focus) cells[0].focus();
}

function hasAnswer(input) {
  return /^-?\d+$/.test(input.value);
}

table.addEventListener("input", event => {
  if (event.isComposing || graded) return;
  const input = event.target;
  const value = input.value.normalize("NFKC").replace(/−/g, "-");
  const sign = operation === 1 && value.startsWith("-") ? "-" : "";
  input.value = sign + value.replace(/\D/g, "").slice(0, 2);
  const filled = cells.filter(hasAnswer).length;
  count.textContent = filled;
  checkButton.disabled = filled === 0;
  if (filled > 0 && started === null) {
    started = performance.now();
    timer = setInterval(updateTime, 200);
    result.textContent = "入力ができたら「採点する」を押してください。";
  }
});

table.addEventListener("keydown", event => {
  if (event.isComposing || event.altKey || event.ctrlKey || event.metaKey) return;
  const index = cells.indexOf(event.target);
  if (index < 0) return;
  const row = Math.floor(index / side);
  const column = index % side;
  let next;
  switch (event.key) {
    case "Enter":
      if (index === cells.length - 1) {
        (checkButton.disabled ? newButton : checkButton).focus();
        event.preventDefault();
        return;
      }
      next = index + 1;
      break;
    case "ArrowLeft": next = column > 0 ? index - 1 : index; break;
    case "ArrowRight": next = column < side - 1 ? index + 1 : index; break;
    case "ArrowUp": next = row > 0 ? index - side : index; break;
    case "ArrowDown": next = row < side - 1 ? index + side : index; break;
    default: return;
  }
  event.preventDefault();
  if (next !== index) {
    cells[next].focus();
    cells[next].select();
  }
});

operations.addEventListener("change", event => {
  operation = Number(event.target.value);
  newBoard(false);
});
newButton.addEventListener("click", () => newBoard(true));
checkButton.addEventListener("click", () => {
  if (graded) return;
  clearInterval(timer);
  updateTime();
  graded = true;
  checkButton.disabled = true;
  let correct = 0;
  let incorrect = 0;
  cells.forEach((input, index) => {
    const mark = wasm.grade(operation, rows[Math.floor(index / side)], columns[index % side],
      Number(hasAnswer(input)), Number(input.value));
    input.readOnly = true;
    if (mark === 2) {
      correct++;
      input.parentElement.dataset.mark = "correct";
    } else if (mark === 1) {
      incorrect++;
      input.parentElement.dataset.mark = "incorrect";
      input.setAttribute("aria-invalid", "true");
    }
  });
  result.textContent = `正解 ${correct}問 ／ 不正解 ${incorrect}問 ／ 未入力 ${cells.length - correct - incorrect}問`;
  result.tabIndex = -1;
  result.focus();
});

try {
  const response = await fetch(new URL("./hundred_math.wasm", import.meta.url));
  if (!response.ok) throw new Error(`WebAssembly: ${response.status}`);
  const module = await WebAssembly.instantiate(await response.arrayBuffer());
  wasm = module.instance.exports;
  operation = Number(operations.querySelector("input:checked").value);
  newBoard(false);
  operations.disabled = false;
  newButton.disabled = false;
  const resetBattle = setupBattle(wasm);
  const battle = document.querySelector("#battle");
  const practice = document.querySelector("#practice-panel");
  const playMode = document.querySelector("#play-mode");
  const practiceMode = document.querySelector("#practice-mode");
  const showPractice = show => {
    if (show) resetBattle();
    battle.hidden = show;
    practice.hidden = !show;
    playMode.setAttribute("aria-pressed", !show);
    practiceMode.setAttribute("aria-pressed", show);
  };
  playMode.addEventListener("click", () => showPractice(false));
  practiceMode.addEventListener("click", () => showPractice(true));
  document.querySelector("#battle-start").disabled = false;
  document.querySelector("#battle-operation").disabled = false;
} catch (error) {
  console.error(error);
  document.querySelector("#question").textContent = "計算機能を読み込めませんでした。";
  result.textContent = "計算機能を読み込めませんでした。ページを再読み込みしてください。";
}
