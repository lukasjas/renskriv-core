// background.js — Laster WASM og handterer skanneforesporsler

let wasmModule = null;

// Last WASM-modulen ved oppstart
async function initWasm() {
  try {
    // Hent URL-er til WASM-filene i utvidelsen
    const jsUrl = browser.runtime.getURL("src/wasm/renskriv_wasm.js");
    const wasmUrl = browser.runtime.getURL("src/wasm/renskriv_wasm_bg.wasm");

    // Importer JS-glue-koden og initialiser WASM
    const mod = await import(jsUrl);
    await mod.default(wasmUrl);
    wasmModule = mod;

    console.log("Renskriv WASM lastet, versjon:", wasmModule.version());
  } catch (err) {
    console.error("Kunne ikke laste WASM:", err);
  }
}

initWasm();

// Klikk pa utvidingsikon => toggle panelet i aktiv fane
browser.browserAction.onClicked.addListener(async (tab) => {
  browser.tabs.sendMessage(tab.id, { type: "TOGGLE_PANEL" });
});

// Lytt etter meldinger fra content script
browser.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === "SCAN_TEXT") {
    handleScan(message.text).then(sendResponse);
    return true; // asynkront svar
  }

  if (message.type === "REDACT_TEXT") {
    const result = applyRedactions(message.text, message.approvedSpans);
    sendResponse({ redactedText: result });
  }

  if (message.type === "GET_STATUS") {
    sendResponse({
      ready: wasmModule !== null,
      version: wasmModule ? wasmModule.version() : null,
    });
  }
});

async function handleScan(text) {
  if (!wasmModule) {
    await initWasm();
  }
  if (!wasmModule) {
    return { error: "WASM ikke lastet" };
  }

  try {
    const spans = wasmModule.scan_text(text);
    return { spans: spans || [] };
  } catch (err) {
    console.error("Skannefeil:", err);
    return { error: err.message };
  }
}

// Erstatt godkjente funn med plassholdere.
// Jobber bakfra slik at posisjoner forblir gyldige.
// Hopper over spans som overlappar med allereie utforte erstatningar.
function applyRedactions(originalText, approvedSpans) {
  const sorted = [...approvedSpans].sort((a, b) => b.start - a.start);
  let result = originalText;
  let appliedEnd = Infinity; // Nedre grense for neste gyldige erstatning

  for (const span of sorted) {
    // Hopp over om denne spanen overlappar med ein allereie erstatta span
    if (span.end > appliedEnd) {
      continue;
    }
    const placeholder = "[" + span.pii_type.toUpperCase() + "]";
    result =
      result.substring(0, span.start) +
      placeholder +
      result.substring(span.end);
    appliedEnd = span.start;
  }
  return result;
}
