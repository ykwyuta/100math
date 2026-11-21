export function readAxes(wasm, seed, digits) {
  return [0, 10].map(start =>
    Array.from({ length: 10 }, (_, index) => wasm.board_number(seed, digits, start + index)));
}

// Keep a selected cell visible without moving the rest of the page.
export function revealCell(cell) {
  const board = cell.closest(".board");
  const view = board.getBoundingClientRect();
  const bounds = cell.getBoundingClientRect();
  if (bounds.right > view.right) board.scrollLeft += bounds.right - view.right;
  const rowHeading = cell.parentElement.firstElementChild;
  const left = getComputedStyle(rowHeading).position === "sticky"
    ? rowHeading.getBoundingClientRect().right : view.left;
  if (bounds.left < left) board.scrollLeft += bounds.left - left;
}
