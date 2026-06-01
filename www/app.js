"use strict";

// Touch UI for rust-audio-cast. Dependency-free: talks to the same-origin REST
// API the BFF serves, polls /status to stay in sync with changes made
// elsewhere (e.g. a phone), and avoids fighting the user mid-drag.
//
// Two views fit the small panel: a compact main view (current speaker +
// transport + volume) and a separate speaker-picker page reached via "Find
// speakers". Picking a speaker returns to the main view.

const POLL_MS = 2000;

const el = {
  viewMain: document.getElementById("view-main"),
  viewSpeakers: document.getElementById("view-speakers"),
  stateBadge: document.getElementById("state-badge"),
  nowonName: document.getElementById("nowon-name"),
  find: document.getElementById("find-btn"),
  back: document.getElementById("back-btn"),
  refresh: document.getElementById("refresh-btn"),
  deviceList: document.getElementById("device-list"),
  play: document.getElementById("play-btn"),
  stop: document.getElementById("stop-btn"),
  mute: document.getElementById("mute-btn"),
  slider: document.getElementById("volume-slider"),
  volumeLabel: document.getElementById("volume-label"),
  toast: document.getElementById("toast"),
};

let devices = [];
let selectedUuid = null;
let selectedName = null;
let muted = false;
let draggingVolume = false;
let toastTimer = null;

// --- API helpers ----------------------------------------------------------

async function api(path, method = "GET") {
  const res = await fetch(path, { method });
  if (!res.ok) {
    let detail = res.statusText;
    try {
      const body = await res.json();
      if (body && body.error) detail = body.error;
    } catch (_) {
      /* non-JSON error body */
    }
    throw new Error(detail);
  }
  const text = await res.text();
  return text ? JSON.parse(text) : null;
}

function toast(message, isError = false) {
  el.toast.textContent = message || "";
  el.toast.classList.toggle("toast--error", isError);
  clearTimeout(toastTimer);
  if (message) {
    toastTimer = setTimeout(() => {
      el.toast.textContent = "";
      el.toast.classList.remove("toast--error");
    }, 4000);
  }
}

async function run(action) {
  try {
    return await action();
  } catch (e) {
    toast(e.message || "Request failed", true);
    return null;
  }
}

// --- View switching -------------------------------------------------------

function showSpeakers() {
  el.viewMain.classList.add("view--hidden");
  el.viewSpeakers.classList.remove("view--hidden");
}

function showMain() {
  el.viewSpeakers.classList.add("view--hidden");
  el.viewMain.classList.remove("view--hidden");
}

// --- Rendering ------------------------------------------------------------

function renderDevices() {
  el.deviceList.innerHTML = "";
  if (devices.length === 0) {
    const li = document.createElement("li");
    li.className = "devices__empty";
    li.textContent = "No speakers found. Tap ⟳ to rescan.";
    el.deviceList.appendChild(li);
    return;
  }
  for (const d of devices) {
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.className = "device";
    btn.setAttribute("aria-selected", String(d.uuid === selectedUuid));
    btn.innerHTML = `
      <span>
        <span class="device__name"></span>
        <span class="device__model"></span>
      </span>
      <span class="device__check">✓</span>`;
    btn.querySelector(".device__name").textContent = d.name;
    btn.querySelector(".device__model").textContent = d.model || "";
    btn.addEventListener("click", () => selectDevice(d.uuid, d.name));
    li.appendChild(btn);
    el.deviceList.appendChild(li);
  }
}

// Tapping a speaker selects it and returns to the main view.
function selectDevice(uuid, name) {
  selectedUuid = uuid;
  selectedName = name || nameForUuid(uuid);
  updateNowOn();
  updateControls();
  renderDevices();
  showMain();
}

function nameForUuid(uuid) {
  const d = devices.find((x) => x.uuid === uuid);
  return d ? d.name : null;
}

function updateNowOn() {
  el.nowonName.textContent = selectedName || "No speaker";
}

function setBadge(state) {
  el.stateBadge.textContent = state;
  el.stateBadge.className =
    "badge " +
    (state === "playing"
      ? "badge--playing"
      : state === "connected"
        ? "badge--connected"
        : "badge--idle");
}

function setVolumeUi(percent) {
  if (!draggingVolume) el.slider.value = String(percent);
  el.volumeLabel.textContent = `${draggingVolume ? el.slider.value : percent}%`;
}

function setMutedUi(isMuted) {
  muted = isMuted;
  el.mute.textContent = isMuted ? "Unmute" : "Mute";
  el.mute.setAttribute("aria-pressed", String(isMuted));
  el.slider.classList.toggle("muted", isMuted);
}

function updateControls(playing) {
  const isPlaying = playing ?? el.stateBadge.textContent === "playing";
  el.play.disabled = !selectedUuid || isPlaying;
  el.stop.disabled = !isPlaying;
}

// --- Data flow ------------------------------------------------------------

async function loadDevices(forceRescan = false) {
  const list = await run(() =>
    forceRescan ? api("/refresh-casts", "POST") : api("/list-casts"),
  );
  if (!list) return;
  devices = list;
  // Refresh the selected speaker's display name if we now know it.
  if (selectedUuid) {
    const name = nameForUuid(selectedUuid);
    if (name) {
      selectedName = name;
      updateNowOn();
    }
  }
  renderDevices();
  updateControls();
}

function applyStatus(status) {
  if (!status) return;
  setBadge(status.state);
  if (status.device) {
    selectedUuid = status.device.uuid;
    selectedName = status.device.name || nameForUuid(selectedUuid);
    updateNowOn();
  }
  setVolumeUi(Math.round((status.volume ?? 0) * 100));
  setMutedUi(!!status.muted);
  updateControls(status.state === "playing");
  renderDevices();
}

async function pollStatus() {
  const status = await run(() => api("/status"));
  applyStatus(status);
}

// --- Event wiring ---------------------------------------------------------

// "Find speakers" opens the picker and kicks off a fresh scan.
el.find.addEventListener("click", () => {
  showSpeakers();
  toast("Scanning…");
  loadDevices(true).then(() => toast(""));
});

el.back.addEventListener("click", showMain);

el.refresh.addEventListener("click", () => {
  toast("Scanning…");
  loadDevices(true).then(() => toast(""));
});

el.play.addEventListener("click", async () => {
  if (!selectedUuid) {
    toast("Pick a speaker first", true);
    return;
  }
  const status = await run(() =>
    api(`/start-cast/${encodeURIComponent(selectedUuid)}`, "POST"),
  );
  applyStatus(status);
});

el.stop.addEventListener("click", async () => {
  const status = await run(() => api("/stop-cast", "POST"));
  applyStatus(status);
});

el.mute.addEventListener("click", async () => {
  const status = await run(() =>
    api(muted ? "/volume-unmute" : "/volume-mute", "POST"),
  );
  applyStatus(status);
});

// Live label while dragging; only send the value on release.
el.slider.addEventListener("input", () => {
  draggingVolume = true;
  el.volumeLabel.textContent = `${el.slider.value}%`;
});

el.slider.addEventListener("change", async () => {
  const level = Number(el.slider.value) / 100;
  draggingVolume = false;
  const res = await run(() => api(`/set-volume/${level.toFixed(2)}`, "POST"));
  if (res && typeof res.volume === "number") {
    setMutedUi(false);
    setVolumeUi(Math.round(res.volume * 100));
  }
});

// --- Boot -----------------------------------------------------------------

updateNowOn();
loadDevices().then(pollStatus);
setInterval(pollStatus, POLL_MS);
