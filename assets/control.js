// Output rows (ROADMAP.md §8.2, R3), delegated: a refresh swaps #live and
// every row. The session cookie and Origin carry each request (§8.5 b, d).
document.addEventListener("click", function (ev) {
  var b = ev.target.closest && ev.target.closest(".ctl button");
  if (!b) return;
  var f = b.closest("fieldset"), t = f.querySelector(".toggle"), on = f.elements.on, off = f.elements.off, out = f.querySelector("output");
  var u = "?station=" + encodeURIComponent(f.dataset.station) + "&output=" + f.dataset.output, m = "POST";
  // From "—" a click switches on: never a surprise power cut.
  if (b === t) u = "/api/control/output" + u + "&state=" + (t.dataset.state === "on" ? "off" : "on");
  else if (b.className === "save") u = "/api/schedule" + u + "&on=" + on.value + "&off=" + off.value;
  else { u = "/api/schedule" + u; m = "DELETE"; }
  out.textContent = "…";
  f.disabled = true;
  fetch(u, { method: m }).then(function (r) {
    return r.json().then(function (j) {
      // 502/504: the switch may have happened; unknown until the next sample.
      if (b === t && (r.ok || r.status === 502 || r.status === 504)) t.textContent = t.dataset.state = r.ok ? j.state : "—";
      if (!r.ok) throw new Error(r.status + " " + j.error);
      if (m === "DELETE") on.value = off.value = "";
      out.textContent = b === t ? "" : m === "DELETE" ? "cleared" : "saved";
    });
  }).catch(function (e) { out.textContent = "Error: " + e.message; }).then(function () { f.disabled = false; });
});
