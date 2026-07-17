// Deterministic Ratatui cell-grid renderer. Renders an exported CellFixture
// (see src/ui/visual_fixture.rs) into a fixed CSS grid. Contains no copy of
// product layout or icon logic: visual drift must originate in the exported
// Ratatui buffer.
"use strict";

const NAMED_COLORS = {
  black: "#000000",
  red: "#cd0000",
  green: "#00cd00",
  yellow: "#cdcd00",
  blue: "#0000ee",
  magenta: "#cd00cd",
  cyan: "#00cdcd",
  gray: "#e5e5e5",
  darkgray: "#7f7f7f",
  lightred: "#ff0000",
  lightgreen: "#00ff00",
  lightyellow: "#ffff00",
  lightblue: "#5c5cff",
  lightmagenta: "#ff00ff",
  lightcyan: "#00ffff",
  white: "#ffffff",
};

function indexedToHex(index) {
  const named = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "gray",
    "darkgray", "lightred", "lightgreen", "lightyellow", "lightblue",
    "lightmagenta", "lightcyan", "white",
  ];
  if (index < 16) {
    return NAMED_COLORS[named[index]];
  }
  if (index < 232) {
    const value = index - 16;
    const steps = [0, 95, 135, 175, 215, 255];
    const r = steps[Math.floor(value / 36)];
    const g = steps[Math.floor((value % 36) / 6)];
    const b = steps[value % 6];
    return `rgb(${r},${g},${b})`;
  }
  const gray = 8 + (index - 232) * 10;
  return `rgb(${gray},${gray},${gray})`;
}

function resolveColor(token, isBackground) {
  if (typeof token !== "string" || token.length === 0) {
    throw new Error(`invalid color token: ${JSON.stringify(token)}`);
  }
  if (token === "reset") {
    return isBackground ? "#000000" : "#d0d0d0";
  }
  if (token.startsWith("rgb(")) {
    return token;
  }
  const indexed = token.match(/^indexed\((\d+)\)$/);
  if (indexed) {
    return indexedToHex(Number(indexed[1]));
  }
  const named = NAMED_COLORS[token];
  if (!named) {
    throw new Error(`unknown color token: ${token}`);
  }
  return named;
}

function renderFixture(fixture) {
  const errorNode = document.getElementById("error");
  errorNode.textContent = "";
  try {
    if (
      !fixture ||
      !Number.isInteger(fixture.width) ||
      !Number.isInteger(fixture.height) ||
      !Array.isArray(fixture.cells)
    ) {
      throw new Error("malformed fixture: width/height/cells required");
    }
    if (fixture.cells.length !== fixture.width * fixture.height) {
      throw new Error(
        `malformed fixture: expected ${fixture.width * fixture.height} cells, got ${fixture.cells.length}`,
      );
    }
    const grid = document.getElementById("grid");
    grid.textContent = "";
    grid.style.gridTemplateColumns = `repeat(${fixture.width}, 10px)`;
    grid.style.gridTemplateRows = `repeat(${fixture.height}, 18px)`;
    for (const cell of fixture.cells) {
      const span = document.createElement("span");
      span.className = "cell";
      span.textContent = cell.symbol;
      span.style.gridColumn = String(cell.x + 1);
      span.style.gridRow = String(cell.y + 1);
      span.style.color = resolveColor(cell.fg, false);
      span.style.background = resolveColor(cell.bg, true);
      const modifiers = cell.modifiers ?? [];
      if (modifiers.includes("bold")) {
        span.style.fontWeight = "700";
      }
      if (modifiers.includes("italic")) {
        span.style.fontStyle = "italic";
      }
      if (modifiers.includes("underlined")) {
        span.style.textDecoration = "underline";
      }
      if (modifiers.includes("dim")) {
        span.style.opacity = "0.6";
      }
      if (modifiers.includes("crossed_out")) {
        span.style.textDecoration = "line-through";
      }
      if (modifiers.includes("reversed")) {
        const fg = span.style.color;
        span.style.color = span.style.background;
        span.style.background = fg;
      }
      grid.appendChild(span);
    }
  } catch (error) {
    errorNode.textContent = String(error);
    throw error;
  }
}

window.renderFixture = renderFixture;
