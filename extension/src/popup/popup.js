// popup.js — Handles user interaction in the popup window

const PII_LABELS = {
  Fodselsnummer: "ID No.",
  Dnummer: "D-No.",
  Phone: "Phone",
  Email: "Email",
  PostalCode: "Postal",
  OrgNumber: "Org No.",
  BankAccount: "Bank Acct",
};

// Element references
const statusEl = document.getElementById("status");
const inputEl = document.getElementById("input-text");
const scanBtn = document.getElementById("scan-btn");
const resultsEl = document.getElementById("results");
const highlightedEl = document.getElementById("highlighted-text");
const spanListEl = document.getElementById("span-list");
const selectAllBtn = document.getElementById("select-all-btn");
const clearAllBtn = document.getElementById("clear-all-btn");
const redactBtn = document.getElementById("redact-btn");
const outputEl = document.getElementById("output");
const redactedTextEl = document.getElementById("redacted-text");
const copyBtn = document.getElementById("copy-btn");

let currentSpans = [];
let currentText = "";

// Open in a separate tab (fullscreen)
const expandBtn = document.getElementById("expand-btn");

// Check if we are already in a tab (not a popup)
if (window.location.search.includes("fullpage")) {
  document.body.classList.add("fullpage");
}

expandBtn.addEventListener("click", () => {
  const url = browser.runtime.getURL("src/popup/popup.html?fullpage");
  browser.tabs.create({ url: url });
  window.close(); // Close popup
});

// Check if WASM is ready
async function checkStatus() {
  try {
    const response = await browser.runtime.sendMessage({ type: "GET_STATUS" });
    if (response.ready) {
      statusEl.textContent = "v" + response.version;
      statusEl.classList.add("ready");
      scanBtn.disabled = false;
    } else {
      statusEl.textContent = "Loading WASM...";
      // Try again shortly
      setTimeout(checkStatus, 500);
    }
  } catch (err) {
    statusEl.textContent = "Error";
    statusEl.classList.add("error");
  }
}

checkStatus();

// Scan button
scanBtn.addEventListener("click", async () => {
  const text = inputEl.value.trim();
  if (!text) return;

  scanBtn.disabled = true;
  scanBtn.textContent = "Scanning...";
  currentText = text;

  try {
    const response = await browser.runtime.sendMessage({
      type: "SCAN_TEXT",
      text: text,
    });

    if (response.error) {
      alert("Error: " + response.error);
      return;
    }

    currentSpans = response.spans || [];

    if (currentSpans.length === 0) {
      resultsEl.classList.remove("hidden");
      highlightedEl.textContent = "No personal data found.";
      spanListEl.innerHTML = "";
      redactBtn.classList.add("hidden");
      selectAllBtn.classList.add("hidden");
      clearAllBtn.classList.add("hidden");
      outputEl.classList.add("hidden");
      return;
    }

    renderResults();
  } catch (err) {
    alert("Scan error: " + err.message);
  } finally {
    scanBtn.disabled = false;
    scanBtn.textContent = "Scan";
  }
});

function renderResults() {
  resultsEl.classList.remove("hidden");
  outputEl.classList.add("hidden");
  redactBtn.classList.remove("hidden");
  selectAllBtn.classList.remove("hidden");
  clearAllBtn.classList.remove("hidden");

  // Build highlighted text
  renderHighlightedText();

  // Build list of matches
  spanListEl.innerHTML = "";
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
    spanListEl.appendChild(item);
  });
}

function renderHighlightedText() {
  // Find which spans are checked
  const checked = currentSpans.filter((_, i) => {
    const cb = document.getElementById("span-" + i);
    return cb && cb.checked;
  });

  // Sort by position
  const sorted = [...checked].sort((a, b) => a.start - b.start);

  // Build HTML with highlights
  highlightedEl.innerHTML = "";
  let pos = 0;

  for (const span of sorted) {
    // Text before span
    if (span.start > pos) {
      highlightedEl.appendChild(
        document.createTextNode(currentText.substring(pos, span.start)),
      );
    }

    const mark = document.createElement("mark");
    mark.className = "pii-" + span.pii_type;
    mark.textContent = currentText.substring(span.start, span.end);
    mark.title = PII_LABELS[span.pii_type] || span.pii_type;
    highlightedEl.appendChild(mark);

    pos = span.end;
  }

  // Rest of the text
  if (pos < currentText.length) {
    highlightedEl.appendChild(
      document.createTextNode(currentText.substring(pos)),
    );
  }
}

// Select all / Clear all
selectAllBtn.addEventListener("click", () => {
  currentSpans.forEach((_, i) => {
    const cb = document.getElementById("span-" + i);
    if (cb) cb.checked = true;
  });
  renderHighlightedText();
});

clearAllBtn.addEventListener("click", () => {
  currentSpans.forEach((_, i) => {
    const cb = document.getElementById("span-" + i);
    if (cb) cb.checked = false;
  });
  renderHighlightedText();
});

// Redact button
redactBtn.addEventListener("click", async () => {
  const approvedSpans = currentSpans.filter((_, i) => {
    const cb = document.getElementById("span-" + i);
    return cb && cb.checked;
  });

  if (approvedSpans.length === 0) {
    alert("No matches selected for redaction.");
    return;
  }

  const response = await browser.runtime.sendMessage({
    type: "REDACT_TEXT",
    text: currentText,
    approvedSpans: approvedSpans,
  });

  outputEl.classList.remove("hidden");
  redactedTextEl.textContent = response.redactedText;
});

// Copy button
copyBtn.addEventListener("click", async () => {
  const text = redactedTextEl.textContent;
  await navigator.clipboard.writeText(text);
  copyBtn.textContent = "Copied!";
  setTimeout(() => {
    copyBtn.textContent = "Copy";
  }, 1500);
});
