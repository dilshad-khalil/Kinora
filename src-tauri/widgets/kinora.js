// Kinora widget SDK. Include with <script src="/kinora.js"></script>.
//
//   Kinora.onSettings((settings) => { ... })   // now and whenever the user changes them
//   Kinora.onVisibility((visible) => { ... })  // false while a fullscreen app is open
//   Kinora.settings                            // the latest settings object
//   Kinora.visible                             // the latest visibility
(function () {
  var settingsFns = [];
  var visibilityFns = [];
  var K = {
    settings: null,
    visible: true,
    onSettings: function (fn) {
      settingsFns.push(fn);
      if (K.settings) fn(K.settings);
    },
    onVisibility: function (fn) {
      visibilityFns.push(fn);
      fn(K.visible);
    },
  };

  window.addEventListener("message", function (e) {
    // Only the page hosting this widget talks to it.
    if (e.source !== window.parent || !e.data || typeof e.data !== "object") return;
    if (e.data.kinora === "settings") {
      K.settings = e.data.settings || {};
      settingsFns.forEach(function (fn) { fn(K.settings); });
    } else if (e.data.kinora === "visible") {
      K.visible = !!e.data.visible;
      visibilityFns.forEach(function (fn) { fn(K.visible); });
    }
  });

  window.Kinora = K;
})();
