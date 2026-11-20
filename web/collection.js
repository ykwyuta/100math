const key = "100math.points.v1";
const maxPoints = 0xffffffff;

function readPoints() {
  const points = Number(localStorage.getItem(key));
  return Number.isInteger(points) && points > 0 ? Math.min(points, maxPoints) : 0;
}

export async function setupCollection(wasm) {
  const panel = document.querySelector("#collection-panel");
  const eraPicker = document.querySelector("#collection-era");
  const catalog = document.querySelector("#coin-catalog");
  const summary = document.querySelector("#collection-summary");
  const battlePoints = document.querySelector("#battle-points");
  const response = await fetch(new URL("./coins/catalog.tsv", import.meta.url));
  if (!response.ok) throw new Error(`Catalog: ${response.status}`);
  const eras = [];
  let count = 0;
  for (const line of (await response.text()).trim().split(/\r?\n/)) {
    if (line.startsWith("# ")) {
      eras.push({ name: line.slice(2), entries: [] });
      continue;
    }
    const [name, dates, description] = line.split("\t");
    eras.at(-1).entries.push({ name, dates, description, index: count++ });
  }
  if (count !== wasm.coin_count(maxPoints)) throw new Error("Unexpected catalog length");
  const entries = eras.flatMap(era => era.entries);
  eras.forEach((era, index) => eraPicker.add(new Option(era.name, index)));
  let total = 0;
  let storageAvailable = true;
  try {
    total = readPoints();
  } catch {
    storageAvailable = false;
  }

  function render() {
    const unlocked = wasm.coin_count(total);
    battlePoints.textContent = `${total.toLocaleString("ja-JP")} pt · ${unlocked}/${count} 枚`;
    summary.textContent = `${total.toLocaleString("ja-JP")} ポイント ／ ${unlocked} / ${count} 枚` +
      (storageAvailable ? "" : "（この端末では保存できません）");
    const era = eras[Number(eraPicker.value)];
    catalog.replaceChildren(...era.entries.map(entry => {
      const card = document.createElement("li");
      const figure = document.createElement("span");
      figure.className = "math-coin";
      figure.setAttribute("aria-hidden", "true");
      const title = document.createElement("h3");
      const dates = document.createElement("p");
      dates.className = "coin-dates";
      const description = document.createElement("p");
      description.textContent = entry.description;
      title.textContent = entry.name;
      dates.textContent = entry.dates;
      if (entry.index < unlocked) {
        const tile = entry.index % 20;
        figure.style.backgroundImage = `url("./coins/atlas-${Math.floor(entry.index / 20)}.webp")`;
        figure.style.backgroundPosition = `${tile % 5 * 25}% ${Math.floor(tile / 5) * 100 / 3}%`;
      } else {
        card.classList.add("coin-locked");
        figure.textContent = "？";
        const locked = document.createElement("span");
        locked.className = "coin-status";
        locked.textContent = "未獲得";
        card.append(locked);
      }
      card.append(figure, title, dates, description);
      return card;
    }));
  }

  async function grant(points) {
    if (!points) return "";
    const save = () => {
      if (storageAvailable) {
        try {
          total = Math.max(total, readPoints());
        } catch {
          storageAvailable = false;
        }
      }
      const before = wasm.coin_count(total);
      total = Math.min(maxPoints, total + points);
      if (storageAvailable) {
        try {
          localStorage.setItem(key, String(total));
        } catch {
          storageAvailable = false;
        }
      }
      const after = wasm.coin_count(total);
      render();
      return after > before ? `+${points} pt！ ${entries[before].name}のコインを獲得。` : `+${points} pt！`;
    };
    return navigator.locks ? navigator.locks.request(key, save) : save();
  }

  window.addEventListener("storage", event => {
    if (event.key !== key) return;
    try {
      total = readPoints();
      render();
    } catch {
      storageAvailable = false;
    }
  });
  eraPicker.addEventListener("change", render);
  render();
  return { grant, panel };
}
