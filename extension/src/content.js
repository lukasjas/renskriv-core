// content.js — Injiserer Renskriv-panelet i sida via Shadow DOM
// + fangar innsending til AI-verktøy og skannar for PII

const PII_LABELS = {
  Fodselsnummer: "Fnr",
  Dnummer: "D-nr",
  Phone: "Telefon",
  Email: "E-post",
  PostalCode: "Postnr",
  OrgNumber: "Orgnr",
  BankAccount: "Bankkonto",
};

let panelOpen = false;
let shadowRoot = null;
let currentSpans = [];
let currentText = "";

// ---- AI-verktøy interception ----

// Kjende AI-sider. Brukt som hint — ikkje einaste mekanisme.
// Selektorar kan brekke når React-komponentar oppdaterast,
// difor bruker vi òg generisk contenteditable/textarea-deteksjon.
const AI_HOSTS = [
  "chatgpt.com",
  "chat.openai.com",
  "claude.ai",
  "gemini.google.com",
  "copilot.microsoft.com",
];

// Er vi på ei AI-side?
function isAISite() {
  return AI_HOSTS.some((h) => location.hostname.endsWith(h));
}

// Hent tekst frå eit input-element (textarea eller contenteditable).
function getInputText(el) {
  if (!el) return "";
  if (el.tagName === "TEXTAREA" || el.tagName === "INPUT") {
    return el.value || "";
  }
  // contenteditable div (ChatGPT, Claude, Gemini brukar dette)
  return el.innerText || "";
}

// Sett tekst tilbake i eit input-element.
// React/frameworks reagerer ikkje på direkte .value-endringar,
// så vi brukar native setter + input-event for å trigge oppdatering.
function setInputText(el, text) {
  if (!el) return;
  if (el.tagName === "TEXTAREA") {
    const setter = Object.getOwnPropertyDescriptor(
      HTMLTextAreaElement.prototype,
      "value",
    ).set;
    setter.call(el, text);
  } else if (el.tagName === "INPUT") {
    const setter = Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    ).set;
    setter.call(el, text);
  } else {
    // contenteditable
    el.innerText = text;
  }
  el.dispatchEvent(new Event("input", { bubbles: true }));
}

// Lim inn tekst i eit element som om brukaren skreiv det.
// Brukar execCommand/InputEvent som React og ProseMirror forstår.
function insertTextAtCursor(el, text) {
  el.focus();
  // execCommand('insertText') fungerer i contenteditable og textarea,
  // og triggar React/ProseMirror sine interne oppdateringar.
  if (document.execCommand("insertText", false, text)) {
    return;
  }
  // Fallback for eldre nettlesarar eller element der execCommand ikkje verkar
  setInputText(el, getInputText(el) + text);
}

// Finn det aktive input-elementet på sida (textarea eller contenteditable).
function findActiveInput() {
  const active = document.activeElement;
  if (!active) return null;

  // Direkte textarea/input
  if (active.tagName === "TEXTAREA" || active.tagName === "INPUT") {
    return active;
  }

  // contenteditable (inkl. nested — gå oppover til næraste contenteditable)
  let el = active;
  while (el && el !== document.body) {
    if (el.isContentEditable) return el;
    el = el.parentElement;
  }

  return null;
}

// Tilstand for intercepta innsending
let interceptedInput = null; // referanse til AI-input-elementet
let interceptMode = false; // true = panel viser intercepta tekst
let skipNextEnter = false; // slepp gjennom Enter etter redaksjon
function setupSubmitInterception() {
  if (!isAISite()) return;

  // 1) Fang paste — blokker innliming, skann, opne panel om PII finst.
  //    Teksten kjem aldri inn i AI-inputfeltet før den er skanna.
  //    Om ingen PII: lim inn manuelt (sidan vi blokkerte default).
  document.addEventListener(
    "paste",
    (e) => {
      // Ikkje fang paste i Renskriv-panelet
      if (shadowRoot && shadowRoot.host.contains(e.target)) return;

      const input = findActiveInput();
      if (!input) return;

      // Hent tekst frå clipboard FØR vi blokkerer
      const pastedText = e.clipboardData?.getData("text/plain") || "";
      if (!pastedText.trim()) return;

      // Blokker innliminga — teksten kjem ikkje inn i inputfeltet
      e.preventDefault();
      e.stopPropagation();

      // Skann teksten
      browser.runtime
        .sendMessage({ type: "SCAN_TEXT", text: pastedText })
        .then((response) => {
          if (response.spans && response.spans.length > 0) {
            // PII funne — opne panelet. Inputfeltet forblir tomt.
            interceptedInput = input;
            interceptMode = true;
            openPanelWithText(pastedText, response.spans);
          } else {
            // Ingen PII — lim inn teksten manuelt sidan vi blokkerte default
            insertTextAtCursor(input, pastedText);
          }
        })
        .catch(() => {
          // Feil — lim inn likevel så brukaren ikkje mistar teksten
          insertTextAtCursor(input, pastedText);
        });
    },
    true,
  );

  // 2) Fang Enter — siste sjekk for manuelt skrive PII
  document.addEventListener(
    "keydown",
    (e) => {
      if (e.key !== "Enter" || e.shiftKey) return;

      // Slepp gjennom Enter etter at vi sjølv har trigga det
      if (skipNextEnter) {
        skipNextEnter = false;
        return;
      }

      // Ikkje fang Enter frå Renskriv-panelet
      if (shadowRoot && shadowRoot.host.contains(e.target)) return;

      // Om panelet allereie er ope i intercept-modus, blokker Enter
      if (interceptMode) {
        e.preventDefault();
        e.stopPropagation();
        return;
      }

      const input = findActiveInput();
      if (!input) return;

      const text = getInputText(input).trim();
      if (!text) return;

      // Blokker Enter medan vi skannar
      e.preventDefault();
      e.stopPropagation();

      browser.runtime
        .sendMessage({ type: "SCAN_TEXT", text: text })
        .then((response) => {
          if (response.spans && response.spans.length > 0) {
            interceptedInput = input;
            interceptMode = true;
            openPanelWithText(text, response.spans);
          } else {
            // Reint — send gjennom
            resubmitEnter(input);
          }
        })
        .catch(() => {
          // Feil — ikkje blokker brukaren
          resubmitEnter(input);
        });
    },
    true,
  );
}

// Send Enter-tasten på nytt til inputfeltet
function resubmitEnter(el) {
  skipNextEnter = true;
  el.focus();
  el.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Enter",
      code: "Enter",
      keyCode: 13,
      which: 13,
      bubbles: true,
      cancelable: true,
    }),
  );
}

// Opne panelet, fyll inn tekst og vis skanneresultat
function openPanelWithText(text, spans) {
  if (!shadowRoot) createPanel();

  // Opne panelet om det ikkje allereie er ope
  if (!panelOpen) {
    panelOpen = true;
    $("panel").classList.add("open");
    shadowRoot.host.style.pointerEvents = "auto";
  }

  // Fyll inn tekst og resultat
  $("input-text").value = text;
  currentText = text;
  currentSpans = spans;

  // Vis "Sladd og send" i staden for berre "Sladd" i intercept-modus
  renderResults();
  updateRedactButton();
}

// Oppdater redact-knappen basert på modus
function updateRedactButton() {
  const btn = $("redact-btn");
  if (!btn) return;
  if (interceptMode) {
    btn.textContent = "Sladd og send";
  } else {
    btn.textContent = "Sladd";
  }
}

// ---- Shadow DOM oppsett ----

function createPanel() {
  const host = document.createElement("div");
  host.id = "renskriv-host";
  // all:initial nullstiller alle arvede stilar fraa sida
  host.style.cssText =
    "all: initial; position: fixed; z-index: 2147483647; pointer-events: none;";
  document.documentElement.appendChild(host);

  shadowRoot = host.attachShadow({ mode: "closed" });

  shadowRoot.innerHTML = `
<style>${PANEL_CSS}</style>
<div class="panel" id="panel">
    <header id="drag-handle">
        <h1>Renskriv</h1>
        <div class="header-right">
            <span class="status" id="status">Laster...</span>
            <button class="close-btn" id="close-btn" title="Lukk">&times;</button>
        </div>
    </header>

    <div class="panel-body">
        <div class="input-section">
            <label for="input-text">Lim inn tekst for skanning:</label>
            <textarea id="input-text" rows="6" placeholder="Lim inn tekst her..."></textarea>
            <button id="scan-btn" class="scan-btn" disabled>Skann</button>
        </div>

        <div id="results" class="results hidden">
            <h2>Funn</h2>
            <div id="highlighted-text" class="highlighted-text"></div>
            <div id="span-list" class="span-list"></div>
            <div class="actions">
                <button id="select-all-btn">Velg alle</button>
                <button id="clear-all-btn">Fjern alle</button>
            </div>
            <button id="redact-btn" class="redact-btn">Sladd</button>
        </div>

        <div id="output" class="output hidden">
            <h2>Sladdet tekst</h2>
            <pre id="redacted-text"></pre>
            <button id="copy-btn">Kopier</button>
        </div>
    </div>

    <div class="edge edge-n" data-dir="n"></div>
    <div class="edge edge-s" data-dir="s"></div>
    <div class="edge edge-w" data-dir="w"></div>
    <div class="edge edge-e" data-dir="e"></div>
    <div class="edge edge-nw" data-dir="nw"></div>
    <div class="edge edge-ne" data-dir="ne"></div>
    <div class="edge edge-sw" data-dir="sw"></div>
    <div class="edge edge-se" data-dir="se"></div>
</div>
`;

  setupEventListeners();
  checkStatus();
}

// ---- Hjelpar: finn element i shadow ----

function $(id) {
  return shadowRoot.getElementById(id);
}

// ---- Vis / skjul panelet ----

function togglePanel() {
  if (!shadowRoot) createPanel();
  panelOpen = !panelOpen;
  $("panel").classList.toggle("open", panelOpen);
  shadowRoot.host.style.pointerEvents = panelOpen ? "auto" : "none";

  // Nullstill intercept-modus når panelet lukkast
  if (!panelOpen) {
    interceptedInput = null;
    interceptMode = false;
    if (shadowRoot) updateRedactButton();
  }
}

// ---- Event listeners ----

function setupEventListeners() {
  // Stopp tastatur-hendingar fraa aa boble ut til sida.
  // Utan dette fangar ChatGPT/Claude/etc. tastetrykkane vaare.
  const panel = $("panel");
  for (const evt of ["keydown", "keyup", "keypress", "input"]) {
    panel.addEventListener(evt, (e) => e.stopPropagation());
  }

  $("close-btn").addEventListener("click", () => togglePanel());
  setupDrag();
  setupResize();

  $("scan-btn").addEventListener("click", handleScan);

  $("select-all-btn").addEventListener("click", () => {
    currentSpans.forEach((_, i) => {
      const cb = $("span-" + i);
      if (cb) cb.checked = true;
    });
    renderHighlightedText();
  });

  $("clear-all-btn").addEventListener("click", () => {
    currentSpans.forEach((_, i) => {
      const cb = $("span-" + i);
      if (cb) cb.checked = false;
    });
    renderHighlightedText();
  });

  $("redact-btn").addEventListener("click", handleRedact);

  $("copy-btn").addEventListener("click", async () => {
    const text = $("redacted-text").textContent;
    await navigator.clipboard.writeText(text);
    $("copy-btn").textContent = "Kopiert!";
    setTimeout(() => {
      $("copy-btn").textContent = "Kopier";
    }, 1500);
  });
}

// ---- Dra panelet (flytt) ----

function setupDrag() {
  const panel = $("panel");
  let dragging = false;
  let offsetX = 0;
  let offsetY = 0;

  // Heile panelet er draggbart, unntatt interaktive element
  const NO_DRAG = new Set(["TEXTAREA", "INPUT", "BUTTON", "MARK", "PRE", "A"]);

  panel.addEventListener("mousedown", (e) => {
    // Ikkje dra fraa interaktive element eller resize-kantar
    if (NO_DRAG.has(e.target.tagName)) return;
    if (e.target.classList.contains("edge")) return;
    dragging = true;
    const rect = panel.getBoundingClientRect();
    offsetX = e.clientX - rect.left;
    offsetY = e.clientY - rect.top;
    panel.classList.add("dragging");
    e.preventDefault();
  });

  document.addEventListener("mousemove", (e) => {
    if (!dragging) return;
    let x = e.clientX - offsetX;
    let y = e.clientY - offsetY;

    // Hald panelet innanfor viewport
    const w = panel.offsetWidth;
    const h = panel.offsetHeight;
    x = Math.max(0, Math.min(x, window.innerWidth - w));
    y = Math.max(0, Math.min(y, window.innerHeight - h));

    panel.style.left = x + "px";
    panel.style.top = y + "px";
    // Fjern right/bottom-posisjonering naar brukar dreg
    panel.style.right = "auto";
    panel.style.bottom = "auto";
  });

  document.addEventListener("mouseup", () => {
    if (dragging) {
      dragging = false;
      panel.classList.remove("dragging");
    }
  });
}

// ---- Resize fraa alle kantar og hjorne ----

function setupResize() {
  const panel = $("panel");
  const MIN_W = 300;
  const MIN_H = 200;
  let resizing = false;
  let dir = "";
  let startX, startY, startRect;

  // Alle edge-element har data-dir attributt
  const edges = shadowRoot.querySelectorAll(".edge");
  edges.forEach((edge) => {
    edge.addEventListener("mousedown", (e) => {
      resizing = true;
      dir = edge.dataset.dir;
      startX = e.clientX;
      startY = e.clientY;
      startRect = panel.getBoundingClientRect();
      panel.classList.add("dragging");
      e.preventDefault();
      e.stopPropagation();
    });
  });

  document.addEventListener("mousemove", (e) => {
    if (!resizing) return;
    const dx = e.clientX - startX;
    const dy = e.clientY - startY;

    let top = startRect.top;
    let left = startRect.left;
    let w = startRect.width;
    let h = startRect.height;

    // Kva kant/hjorne blir drege?
    if (dir.includes("e")) w = Math.max(MIN_W, w + dx);
    if (dir.includes("s")) h = Math.max(MIN_H, h + dy);
    if (dir.includes("w")) {
      const newW = Math.max(MIN_W, w - dx);
      left = left + (w - newW);
      w = newW;
    }
    if (dir.includes("n")) {
      const newH = Math.max(MIN_H, h - dy);
      top = top + (h - newH);
      h = newH;
    }

    panel.style.left = left + "px";
    panel.style.top = top + "px";
    panel.style.right = "auto";
    panel.style.width = w + "px";
    panel.style.height = h + "px";
  });

  document.addEventListener("mouseup", () => {
    if (resizing) {
      resizing = false;
      panel.classList.remove("dragging");
    }
  });
}

// ---- WASM-status ----

async function checkStatus() {
  try {
    const response = await browser.runtime.sendMessage({ type: "GET_STATUS" });
    if (response.ready) {
      $("status").textContent = "v" + response.version;
      $("status").classList.add("ready");
      $("scan-btn").disabled = false;
    } else {
      $("status").textContent = "Laster WASM...";
      setTimeout(checkStatus, 500);
    }
  } catch (err) {
    $("status").textContent = "Feil";
    $("status").classList.add("error");
  }
}

// ---- Skanning ----

async function handleScan() {
  const text = $("input-text").value.trim();
  if (!text) return;

  $("scan-btn").disabled = true;
  $("scan-btn").textContent = "Skanner...";
  currentText = text;

  try {
    const response = await browser.runtime.sendMessage({
      type: "SCAN_TEXT",
      text: text,
    });

    if (response.error) {
      alert("Feil: " + response.error);
      return;
    }

    currentSpans = response.spans || [];

    if (currentSpans.length === 0) {
      $("results").classList.remove("hidden");
      $("highlighted-text").textContent = "Ingen personopplysningar funne.";
      $("span-list").innerHTML = "";
      $("redact-btn").classList.add("hidden");
      $("select-all-btn").classList.add("hidden");
      $("clear-all-btn").classList.add("hidden");
      $("output").classList.add("hidden");
      return;
    }

    renderResults();
  } catch (err) {
    alert("Feil ved skanning: " + err.message);
  } finally {
    $("scan-btn").disabled = false;
    $("scan-btn").textContent = "Skann";
  }
}

// ---- Resultat-rendering ----

function renderResults() {
  $("results").classList.remove("hidden");
  $("output").classList.add("hidden");
  $("redact-btn").classList.remove("hidden");
  $("select-all-btn").classList.remove("hidden");
  $("clear-all-btn").classList.remove("hidden");

  renderHighlightedText();

  const list = $("span-list");
  list.innerHTML = "";
  currentSpans.forEach((span, i) => {
    const item = document.createElement("div");
    item.className = "span-item";

    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = true;
    checkbox.id = "span-" + i;
    checkbox.addEventListener("change", renderHighlightedText);

    const value = document.createElement("span");
    value.className = "span-value";
    value.textContent = span.value;

    const type = document.createElement("span");
    type.className = "span-type type-" + span.pii_type;
    type.textContent = PII_LABELS[span.pii_type] || span.pii_type;

    const confidence = document.createElement("span");
    confidence.className = "span-confidence";
    confidence.textContent = Math.round(span.confidence * 100) + "%";

    item.appendChild(checkbox);
    item.appendChild(value);
    item.appendChild(type);
    item.appendChild(confidence);
    list.appendChild(item);
  });
}

function renderHighlightedText() {
  const checked = currentSpans.filter((_, i) => {
    const cb = $("span-" + i);
    return cb && cb.checked;
  });

  const sorted = [...checked].sort((a, b) => a.start - b.start);

  const el = $("highlighted-text");
  el.innerHTML = "";
  let pos = 0;

  for (const span of sorted) {
    if (span.start > pos) {
      el.appendChild(
        document.createTextNode(currentText.substring(pos, span.start)),
      );
    }

    const mark = document.createElement("mark");
    mark.className = "pii-" + span.pii_type;
    mark.textContent = currentText.substring(span.start, span.end);
    mark.title = PII_LABELS[span.pii_type] || span.pii_type;
    el.appendChild(mark);

    pos = span.end;
  }

  if (pos < currentText.length) {
    el.appendChild(document.createTextNode(currentText.substring(pos)));
  }
}

// ---- Sladding ----

async function handleRedact() {
  const approvedSpans = currentSpans.filter((_, i) => {
    const cb = $("span-" + i);
    return cb && cb.checked;
  });

  if (approvedSpans.length === 0) {
    alert("Ingen funn er valt for sladding.");
    return;
  }

  const response = await browser.runtime.sendMessage({
    type: "REDACT_TEXT",
    text: currentText,
    approvedSpans: approvedSpans,
  });

  if (interceptMode && interceptedInput) {
    // Intercept-modus: erstatt teksten i AI-inputfeltet og send
    const target = interceptedInput;
    setInputText(target, response.redactedText);

    // Lukk panelet
    panelOpen = false;
    $("panel").classList.remove("open");
    shadowRoot.host.style.pointerEvents = "none";

    // Nullstill intercept-tilstand FØR re-submit
    interceptedInput = null;
    interceptMode = false;
    updateRedactButton();

    // Kort forseinking så React rekk å oppdatere, deretter send
    setTimeout(() => {
      resubmitEnter(target);
    }, 100);
  } else {
    // Vanleg modus: vis sladda tekst i panelet
    $("output").classList.remove("hidden");
    $("redacted-text").textContent = response.redactedText;
  }
}

// ---- Lytt etter melding fraa background (toggle panel) ----

browser.runtime.onMessage.addListener((message) => {
  if (message.type === "TOGGLE_PANEL") {
    togglePanel();
  }
});

// ---- Start interception på AI-sider ----

setupSubmitInterception();

// ---- CSS (isolert i Shadow DOM) ----

const PANEL_CSS = `
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

.panel {
    position: fixed;
    top: 16px;
    right: 16px;
    width: 400px;
    height: 520px;
    background: #fafafa;
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.18), 0 0 0 1px rgba(0, 0, 0, 0.06);
    display: flex;
    flex-direction: column;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-size: 13px;
    color: #1a1a1a;
    line-height: 1.4;
    opacity: 0;
    transform: scale(0.95) translateY(-8px);
    pointer-events: none;
    transition: opacity 0.2s ease, transform 0.2s ease;
    overflow: hidden;
    cursor: grab;
}

.panel.dragging {
    cursor: grabbing;
}

.panel.open {
    opacity: 1;
    transform: scale(1) translateY(0);
    pointer-events: auto;
}

.panel.dragging {
    transition: none;
    user-select: none;
}

header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: #1a365d;
    color: #fff;
    flex-shrink: 0;
    border-radius: 10px 10px 0 0;
    user-select: none;
}

textarea, input, button, select {
    cursor: auto;
}

.header-right {
    display: flex;
    align-items: center;
    gap: 8px;
}

h1 {
    font-size: 14px;
    font-weight: 600;
    color: #fff;
}

h2 {
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 8px;
}

.status {
    font-size: 10px;
    padding: 2px 8px;
    border-radius: 10px;
    background: rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.8);
}

.status.ready {
    background: rgba(34, 197, 94, 0.3);
    color: #bbf7d0;
}

.status.error {
    background: rgba(239, 68, 68, 0.3);
    color: #fecaca;
}

.close-btn {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
    color: rgba(255, 255, 255, 0.7);
    padding: 0 2px;
    line-height: 1;
}

.close-btn:hover {
    color: #fff;
    background: none;
}

.panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 16px;
}

label {
    display: block;
    font-size: 12px;
    color: #475569;
    margin-bottom: 4px;
}

textarea {
    width: 100%;
    padding: 8px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    font-family: inherit;
    font-size: 13px;
    resize: vertical;
    margin-bottom: 8px;
    line-height: 1.4;
}

textarea:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.15);
}

button {
    padding: 6px 14px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: #fff;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
}

button:hover {
    background: #f1f5f9;
}

button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.scan-btn {
    background: #1a365d;
    color: #fff;
    border-color: #1a365d;
}

.scan-btn:hover:not(:disabled) {
    background: #2d4a7a;
}

.results {
    margin-top: 12px;
    border-top: 1px solid #e2e8f0;
    padding-top: 10px;
}

.highlighted-text {
    padding: 8px;
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-size: 13px;
    line-height: 1.6;
    margin-bottom: 10px;
    max-height: 150px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
}

.span-list {
    margin-bottom: 8px;
}

.span-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    border-bottom: 1px solid #f1f5f9;
    font-size: 12px;
}

.span-item input[type="checkbox"] {
    flex-shrink: 0;
}

.span-value {
    font-family: monospace;
    font-weight: 500;
}

.span-type {
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 11px;
    color: #fff;
    white-space: nowrap;
}

.span-confidence {
    color: #94a3b8;
    font-size: 11px;
    margin-left: auto;
}

.actions {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
}

.redact-btn {
    width: 100%;
    background: #dc2626;
    color: #fff;
    border-color: #dc2626;
    font-weight: 500;
}

.redact-btn:hover {
    background: #b91c1c;
}

.output {
    margin-top: 12px;
    border-top: 1px solid #e2e8f0;
    padding-top: 10px;
}

#redacted-text {
    padding: 8px;
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    font-family: inherit;
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    margin-bottom: 8px;
    max-height: 150px;
    overflow-y: auto;
}

#copy-btn {
    width: 100%;
}

.hidden {
    display: none;
}

/* Resize-kantar og hjorne (usynlege, berre markør endrar seg) */
.edge {
    position: absolute;
}

.edge-n  { top: -3px;    left: 6px;    right: 6px;   height: 6px; cursor: ns-resize; }
.edge-s  { bottom: -3px; left: 6px;    right: 6px;   height: 6px; cursor: ns-resize; }
.edge-w  { left: -3px;   top: 6px;     bottom: 6px;  width: 6px;  cursor: ew-resize; }
.edge-e  { right: -3px;  top: 6px;     bottom: 6px;  width: 6px;  cursor: ew-resize; }
.edge-nw { top: -3px;    left: -3px;   width: 10px;  height: 10px; cursor: nwse-resize; }
.edge-ne { top: -3px;    right: -3px;  width: 10px;  height: 10px; cursor: nesw-resize; }
.edge-sw { bottom: -3px; left: -3px;   width: 10px;  height: 10px; cursor: nesw-resize; }
.edge-se { bottom: -3px; right: -3px;  width: 10px;  height: 10px; cursor: nwse-resize; }

mark {
    padding: 1px 2px;
    border-radius: 2px;
}

.pii-Fodselsnummer { background: rgba(239, 68, 68, 0.2); }
.pii-Dnummer { background: rgba(239, 68, 68, 0.2); }
.pii-Phone { background: rgba(249, 115, 22, 0.2); }
.pii-Email { background: rgba(34, 197, 94, 0.2); }
.pii-PostalCode { background: rgba(59, 130, 246, 0.2); }
.pii-OrgNumber { background: rgba(139, 92, 246, 0.2); }
.pii-BankAccount { background: rgba(234, 179, 8, 0.2); }

.type-Fodselsnummer { background: #ef4444; }
.type-Dnummer { background: #ef4444; }
.type-Phone { background: #f97316; }
.type-Email { background: #22c55e; }
.type-PostalCode { background: #3b82f6; }
.type-OrgNumber { background: #8b5cf6; }
.type-BankAccount { background: #eab308; }
`;
