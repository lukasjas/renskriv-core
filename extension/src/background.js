// background.js — Loads WASM and handles scan requests

let wasmModule = null;

// Load the WASM module on startup
async function initWasm() {
  try {
    // Get URLs for the WASM files in the extension
    const jsUrl = browser.runtime.getURL("src/wasm/renskriv_wasm.js");
    const wasmUrl = browser.runtime.getURL("src/wasm/renskriv_wasm_bg.wasm");

    // Import JS glue code and initialize WASM
    const mod = await import(jsUrl);
    await mod.default(wasmUrl);
    wasmModule = mod;

    console.log("Renskriv WASM loaded, version:", wasmModule.version());
  } catch (err) {
    console.error("Failed to load WASM:", err);
  }
}

initWasm();

// Click on extension icon => toggle the panel in the active tab.
// The content script is only preloaded on AI sites (see manifest.json).
// Elsewhere we inject it on demand — the click grants activeTab for this tab.
browser.browserAction.onClicked.addListener(async (tab) => {
  try {
    await browser.tabs.sendMessage(tab.id, { type: "TOGGLE_PANEL" });
  } catch {
    try {
      await browser.tabs.executeScript(tab.id, { file: "/src/content.js" });
      await browser.tabs.sendMessage(tab.id, { type: "TOGGLE_PANEL" });
    } catch (err) {
      // Privileged pages (about:, addons.mozilla.org) can't be scripted
      console.error("Could not open panel on this page:", err);
    }
  }
});

// Listen for messages from content script
browser.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === "SCAN_TEXT") {
    handleScan(message.text).then(sendResponse);
    return true; // async response
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
    return { error: "WASM not loaded" };
  }

  try {
    const spans = wasmModule.scan_text(text);
    return { spans: spans || [] };
  } catch (err) {
    console.error("Scan error:", err);
    return { error: err.message };
  }
}

// Replace approved matches with placeholders.
// Works backwards so that positions remain valid.
// Skips spans that overlap with already-applied replacements.
function applyRedactions(originalText, approvedSpans) {
  const sorted = [...approvedSpans].sort((a, b) => b.start - a.start);
  let result = originalText;
  let appliedEnd = Infinity; // Lower bound for next valid replacement

  for (const span of sorted) {
    // Skip if this span overlaps with an already-replaced span
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
