"use strict";

const byId = id => document.getElementById(id);
const tabs = [...document.querySelectorAll('[role="tab"]')];
const devices = [...document.querySelectorAll("[data-device]")];
let activeTab = "file";
let selected = { name: "旅行照片.zip", count: 1, size: 128 * 1024 * 1024 };
let timer = null;
let pendingDevice = null;
let bookcasePaired = false;
let activeDeviceButton = null;

function refreshSwatches() {
  const style = getComputedStyle(document.documentElement);
  document.querySelectorAll("[data-color]").forEach(element => {
    element.textContent = style.getPropertyValue(`--sw-color-${element.dataset.color}`).trim();
  });
}
document.querySelectorAll("[data-set-theme]").forEach(button => {
  button.addEventListener("click", () => {
    document.documentElement.dataset.theme = button.dataset.setTheme;
    document.querySelectorAll("[data-set-theme]").forEach(other => other.setAttribute("aria-pressed", String(other === button)));
    refreshSwatches();
  });
});
refreshSwatches();

function activateTab(tab) {
  activeTab = tab.id === "file-tab" ? "file" : "text";
  tabs.forEach(other => {
    const active = other === tab;
    other.setAttribute("aria-selected", String(active));
    other.tabIndex = active ? 0 : -1;
    byId(other.getAttribute("aria-controls")).hidden = !active;
  });
}
tabs.forEach((tab, index) => {
  tab.addEventListener("click", () => activateTab(tab));
  tab.addEventListener("keydown", event => {
    let next;
    if (event.key === "ArrowRight") next = tabs[(index + 1) % tabs.length];
    if (event.key === "ArrowLeft") next = tabs[(index - 1 + tabs.length) % tabs.length];
    if (event.key === "Home") next = tabs[0];
    if (event.key === "End") next = tabs.at(-1);
    if (next) { event.preventDefault(); activateTab(next); next.focus(); }
  });
});

function message(text, state = "") {
  byId("transfer-message").textContent = text;
  byId("transfer-message").className = `transfer-message ${state}`.trim();
}
function sizeLabel(size) {
  if (size < 1024) return `${size} B`;
  if (size < 1024 ** 2) return `${(size / 1024).toFixed(1)} KB`;
  if (size < 1024 ** 3) return `${(size / 1024 ** 2).toFixed(1)} MB`;
  return `${(size / 1024 ** 3).toFixed(1)} GB`;
}
function selectFiles(files, folder = false) {
  if (!files.length) return;
  const name = folder && files[0].webkitRelativePath ? files[0].webkitRelativePath.split("/")[0] : files[0].name;
  selected = { name: files.length > 1 && !folder ? `${name} 等 ${files.length} 个文件` : name, count: files.length, size: [...files].reduce((total, file) => total + file.size, 0) };
  byId("selected-file").hidden = false;
  byId("selection-name").textContent = selected.name;
  byId("selection-details").textContent = `${sizeLabel(selected.size)} · 已选 ${selected.count} 个文件`;
  message("已选好内容，点击设备就能发送。");
}
byId("choose-file").addEventListener("click", () => { byId("file-input").value = ""; byId("file-input").click(); });
byId("choose-folder").addEventListener("click", () => { byId("folder-input").value = ""; byId("folder-input").click(); });
byId("file-input").addEventListener("change", event => selectFiles(event.target.files));
byId("folder-input").addEventListener("change", event => selectFiles(event.target.files, true));
byId("clear-selection").addEventListener("click", () => { selected = null; byId("selected-file").hidden = true; message("选择一些内容，再点击接收设备。"); byId("choose-file").focus(); });
byId("dropzone").addEventListener("dragover", event => { event.preventDefault(); byId("dropzone").classList.add("dragging"); });
byId("dropzone").addEventListener("dragleave", () => byId("dropzone").classList.remove("dragging"));
byId("dropzone").addEventListener("drop", event => { event.preventDefault(); byId("dropzone").classList.remove("dragging"); selectFiles(event.dataTransfer.files); });

function hasContent() {
  if (activeTab === "file" && !selected) { message("先选择文件，再点击设备。", "error"); byId("choose-file").focus(); return false; }
  if (activeTab === "text") {
    const text = byId("text-content").value;
    if (!text.trim()) { message("写一些文字，再点击设备。", "error"); byId("text-content").focus(); return false; }
    if (new TextEncoder().encode(text).length > 1024 ** 2) { message("这段文字超过 1 MiB，请缩短后发送。", "error"); byId("text-content").focus(); return false; }
  }
  return true;
}
function setBusy(busy) {
  [...devices, byId("choose-file"), byId("choose-folder"), byId("clear-selection"), ...tabs].forEach(button => button.disabled = busy);
  byId("text-content").disabled = busy;
  byId("transfer-progress").hidden = !busy;
}
function simulateTransfer(device) {
  const contentName = activeTab === "text" ? "一段文字" : selected.name;
  activeDeviceButton = devices.find(button => button.dataset.device === device);
  let progress = 0;
  setBusy(true);
  byId("progress-bar").value = 0;
  byId("progress-label").textContent = "0%";
  byId("transfer-label").textContent = `正在发给${device}`;
  message("正在演示传输进度，内容不会通过网络发送。");
  byId("cancel-transfer").focus();
  timer = setInterval(() => {
    progress = Math.min(progress + 10, 100);
    byId("progress-bar").value = progress;
    byId("progress-label").textContent = `${progress}%`;
    if (progress === 100) {
      const restoreFocus = document.activeElement === byId("cancel-transfer");
      clearInterval(timer); timer = null; setBusy(false);
      message(`嗖，已送到${device}。这是一次演示。`, "done");
      byId("history-name").textContent = contentName;
      byId("history-details").textContent = `发给${device} · 刚刚`;
      byId("history-status").textContent = "已发送";
      if (restoreFocus) activeDeviceButton.focus();
    }
  }, 200);
}
devices.forEach(button => button.addEventListener("click", () => {
  if (!hasContent() || timer) return;
  const device = button.dataset.device;
  if (device === "书房电脑" && !bookcasePaired) { pendingDevice = device; byId("pair-dialog").showModal(); }
  else simulateTransfer(device);
}));
byId("cancel-transfer").addEventListener("click", () => {
  if (timer) clearInterval(timer);
  timer = null; setBusy(false); message("已取消这次演示，可以重新发送。");
  activeDeviceButton?.focus();
});
byId("remember-device").addEventListener("change", event => {
  byId("auto-receive").disabled = !event.target.checked;
  if (!event.target.checked) byId("auto-receive").checked = false;
});
byId("confirm-pair").addEventListener("click", () => {
  bookcasePaired = byId("remember-device").checked;
  if (bookcasePaired) devices[1].querySelector(".device-info > span").textContent = "Windows · 已配对";
  byId("pair-dialog").close();
  if (pendingDevice && hasContent()) simulateTransfer(pendingDevice);
  pendingDevice = null;
});
byId("connect-button").addEventListener("click", () => byId("connect-dialog").showModal());
document.querySelectorAll("[data-close-dialog]").forEach(button => button.addEventListener("click", () => byId(button.dataset.closeDialog).close()));
byId("select-address").addEventListener("click", () => {
  byId("example-address").focus(); byId("example-address").select();
  byId("address-hint").textContent = "地址已选中，可用系统复制操作复制。";
});
window.addEventListener("pagehide", () => { if (timer) clearInterval(timer); });
