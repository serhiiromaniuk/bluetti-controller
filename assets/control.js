// Output rows (ROADMAP.md §8.2, R3), delegated: a refresh swaps #live and
// every row. The session cookie and Origin carry each request (§8.5 b, d).
// A12: busy rows ignore clicks; replies find the live row.
var pending = {};
document.addEventListener("click", function (ev) {
  var b = ev.target.closest && ev.target.closest(".ctl button");
  if (!b) return;
  var f = b.closest("fieldset"), d = f.dataset, key = d.station + "/" + d.output;
  if (pending[key]) return;
  pending[key] = true;
  var row = function () {
    var a = document.querySelectorAll("#live .ctl"), i = a.length;
    while (i--) if (a[i].dataset.station === d.station && a[i].dataset.output === d.output) return a[i];
    return f;
  };
  var t = f.querySelector(".toggle"), on = f.elements.on, off = f.elements.off, say = function (x) { row().querySelector("output").textContent = x; };
  var u = "?station=" + encodeURIComponent(d.station) + "&output=" + d.output, m = "POST";
  // From "—" a click switches on: no surprise power cut.
  if (b === t) u = "/api/control/output" + u + "&state=" + (t.dataset.state === "on" ? "off" : "on");
  else if (b.className === "save") u = "/api/schedule" + u + "&on=" + on.value + "&off=" + off.value;
  else { u = "/api/schedule" + u; m = "DELETE"; }
  say("…");
  f.disabled = true;
  fetch(u, { method: m }).then(function (r) {
    return r.json().then(function (j) {
      var g = row(), gt = g.querySelector(".toggle");
      // 502/504: the switch may have happened; unknown until the next sample.
      if (b === t && (r.ok || r.status === 502 || r.status === 504)) gt.textContent = gt.dataset.state = r.ok ? j.state : "—";
      if (!r.ok) throw new Error(r.status + " " + j.error);
      if (m === "DELETE") g.elements.on.value = g.elements.off.value = "";
      say(b === t ? "" : m === "DELETE" ? "cleared" : "saved");
    });
  }).catch(function (e) { say("Error: " + e.message); }).then(function () {
    row().disabled = false;
    delete pending[key];
  });
});
