// Light/dark theme (ROADMAP.md §8.5 f: no inline script under the CSP).
// Loaded synchronously in <head>, so the saved theme applies before paint.
(function () {
  var r = document.documentElement;
  try { var t = localStorage.getItem("theme"); if (t) r.dataset.theme = t; } catch (e) {}
  r.className = "js";
  addEventListener("DOMContentLoaded", function () {
    var b = document.getElementById("theme");
    if (b) b.addEventListener("click", function () {
      var t = r.dataset.theme == "light" ? "dark" : "light";
      r.dataset.theme = t;
      try { localStorage.setItem("theme", t); } catch (e) {}
      dispatchEvent(new Event("themechange"));
    });
  });
})();
