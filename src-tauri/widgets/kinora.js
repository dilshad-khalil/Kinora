// Kinora widget SDK. Include with <script src="/kinora.js"></script>.
//
//   Kinora.onSettings((settings) => { ... })   // now and whenever the user changes them
//   Kinora.onVisibility((visible) => { ... })  // false while a fullscreen app is open
//   Kinora.on("system", (data) => { ... })     // live data; needs "system" in permissions
//   Kinora.on("media", (data) => { ... })      // now playing (or null); needs "media"
//   Kinora.notify("Title", "Body")             // Windows notification; needs "notifications"
//   Kinora.onError((message) => { ... })       // e.g. a notification was refused
//   Kinora.settings / .visible / .permissions / .data[topic]   // latest values
(function () {
  var settingsFns = [];
  var visibilityFns = [];
  var errorFns = [];
  var dataFns = {};
  var K = {
    settings: null,
    visible: true,
    permissions: [],
    data: {},
    onSettings: function (fn) {
      settingsFns.push(fn);
      if (K.settings) fn(K.settings);
    },
    onVisibility: function (fn) {
      visibilityFns.push(fn);
      fn(K.visible);
    },
    on: function (topic, fn) {
      (dataFns[topic] = dataFns[topic] || []).push(fn);
      if (topic in K.data) fn(K.data[topic]);
    },
    notify: function (title, body) {
      window.parent.postMessage({ kinora: "notify", title: String(title || ""), body: String(body || "") }, "*");
    },
    onError: function (fn) {
      errorFns.push(fn);
    },
  };

  window.addEventListener("message", function (e) {
    // Only the page hosting this widget talks to it.
    var m = e.data;
    if (e.source !== window.parent || !m || typeof m !== "object") return;
    if (m.kinora === "settings") {
      K.settings = m.settings || {};
      settingsFns.forEach(function (fn) { fn(K.settings); });
    } else if (m.kinora === "visible") {
      K.visible = !!m.visible;
      visibilityFns.forEach(function (fn) { fn(K.visible); });
    } else if (m.kinora === "data") {
      K.data[m.topic] = m.data;
      (dataFns[m.topic] || []).forEach(function (fn) { fn(m.data); });
    } else if (m.kinora === "permissions") {
      K.permissions = m.permissions || [];
    } else if (m.kinora === "error") {
      if (!errorFns.length) console.warn("Kinora:", m.message);
      errorFns.forEach(function (fn) { fn(m.message); });
    }
  });

  window.Kinora = K;
})();
