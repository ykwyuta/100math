import { readAxes } from "./board.js";
import { setupBattle } from "./battle.js";
import { setupCollection } from "./collection.js";

const side = 10;
const symbols = ["+", "−", "×"];
const table = document.querySelector("#grid");
const operations = document.querySelector(".operations");
const digitsPicker = document.querySelector("#practice-digits");
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
  result.textContent = "こたえを いれると、タイムが はじまるよ。";
  checkButton.disabled = true;

  const random = new Uint32Array(1);
  do {
    crypto.getRandomValues(random);
  } while (random[0] === previousSeed);
  previousSeed = random[0];
  const digits = Number(digitsPicker.value);
  [rows, columns] = readAxes(wasm, random[0], digits);
  table.dataset.digits = digits;
  table.closest(".board").scrollLeft = 0;

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
      input.setAttribute("aria-label", `${row + 1}ぎょう ${column + 1}れつ：${left} ${symbols[operation]} ${top}`);
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
  input.value = sign + value.replace(/\D/g, "").slice(0, 6);
  const filled = cells.filter(hasAnswer).length;
  count.textContent = filled;
  checkButton.disabled = filled === 0;
  if (filled > 0 && started === null) {
    started = performance.now();
    timer = setInterval(updateTime, 200);
    result.textContent = "こたえを いれたら「さいてん」を おしてね。";
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
digitsPicker.addEventListener("change", () => newBoard(false));
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
  result.textContent = `せいかい ${correct}もん ／ まちがい ${incorrect}もん ／ まだ ${cells.length - correct - incorrect}もん`;
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
  digitsPicker.disabled = false;
  newButton.disabled = false;
  const collection = await setupCollection(wasm);
  const resetBattle = setupBattle(wasm, collection.grant);
  const modes = [
    [document.querySelector("#home-mode"), document.querySelector("#home-panel")],
    [document.querySelector("#play-mode"), document.querySelector("#battle")],
    [document.querySelector("#practice-mode"), document.querySelector("#practice-panel")],
    [document.querySelector("#collection-mode"), collection.panel],
  ];
  function showMode(panel) {
    if (!panel.hidden) return;
    if (panel !== modes[1][1]) resetBattle();
    for (const [option, section] of modes) {
      section.hidden = section !== panel;
      option.setAttribute("aria-pressed", section === panel);
    }
  }
  for (const [button, panel] of modes) {
    button.addEventListener("click", () => showMode(panel));
  }
  for (const [index, id] of ["#start-play", "#start-practice", "#start-collection"].entries()) {
    const button = document.querySelector(id);
    const [tab, panel] = modes[index + 1];
    button.disabled = false;
    button.addEventListener("click", () => {
      showMode(panel);
      tab.focus();
    });
  }
  document.querySelector("#start-status").hidden = true;
  document.querySelector("#collection-mode").disabled = false;
  document.querySelector("#battle-start").disabled = false;
  document.querySelector("#battle-operation").disabled = false;
  document.querySelector("#battle-digits").disabled = false;
} catch (error) {
  console.error(error);
  document.querySelector("#start-status").textContent = "よみこめなかったよ。ページを ひらきなおしてね。";
  document.querySelector("#question").textContent = "けいさんの じゅんびが できなかったよ。";
  result.textContent = "けいさんの じゅんびが できなかったよ。ページを ひらきなおしてね。";
}
