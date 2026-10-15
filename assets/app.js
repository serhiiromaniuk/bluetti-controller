// bluetti-controller front end (ROADMAP.md §8.4 j). Vanilla, no build step,
// no external URL: the page must work offline in an outage.
//
// DOM contract (the templates provide; every part is optional):
//   <div class="chart" data-chart-station="NAME"></div>
//       one per station: SOC (right axis) + the four powers (left axis)
//       from /api/series over ?range=24h|7d (anything else -> 24h).
//   <a data-range="24h">, <a data-range="7d">
//       the range toggle; the current one gets class "on".
//   <form id="pause"> <input name="minutes"> [<input name="token">]
//       <output id="pause-reply"></output> </form>
//       POST /api/polling/pause?minutes=N; Bearer token only when the
//       token field exists and is filled.
// SSE sample / grid_event / poll_failure on /events -> reload, debounced 2 s.
(function () {
  "use strict";
  var range = new URLSearchParams(location.search).get("range") === "7d" ? "7d" : "24h";
  var span = range === "7d" ? 7 * 86400 : 86400;
  var SERIES = [
    ["battery_soc", "SOC %", "#2a7", "%"],
    ["ac_input_power", "AC in W", "#27c", "W"],
    ["dc_input_power", "DC in W", "#8a2", "W"],
    ["ac_output_power", "AC out W", "#c42", "W"],
    ["dc_output_power", "DC out W", "#d80", "W"]
  ];

  document.querySelectorAll("[data-range]").forEach(function (a) {
    if (a.getAttribute("data-range") === range) a.classList.add("on");
  });

  function series(station, metric, from, to) {
    var q = "station=" + encodeURIComponent(station) + "&metric=" + metric +
      "&from=" + from + "&to=" + to + (range === "7d" ? "&step=600" : "");
    return fetch("/api/series?" + q).then(function (r) {
      return r.json().then(function (body) {
        if (!r.ok) throw new Error(metric + ": " + (body.error || r.status));
        return body;
      });
    });
  }

  // The series may differ in timestamps (missing values): join on the union.
  function align(all) {
    var ts = {};
    all.forEach(function (s) { s.t.forEach(function (t) { ts[t] = 1; }); });
    var t = Object.keys(ts).map(Number).sort(function (a, b) { return a - b; });
    return [t].concat(all.map(function (s) {
      var at = {};
      s.t.forEach(function (x, i) { at[x] = s.v[i]; });
      return t.map(function (x) { return x in at ? at[x] : null; });
    }));
  }

  function chart(el) {
    var station = el.getAttribute("data-chart-station");
    var to = Math.floor(Date.now() / 1000);
    Promise.all(SERIES.map(function (s) {
      return series(station, s[0], to - span, to);
    })).then(function (all) {
      new uPlot({
        width: el.clientWidth || 600,
        height: 220,
        scales: { "%": { range: [0, 100] } },
        series: [{}].concat(SERIES.map(function (s) {
          return { label: s[1], stroke: s[2], scale: s[3], width: 1.5 };
        })),
        axes: [{}, { scale: "W" }, { scale: "%", side: 1, grid: { show: false } }]
      }, align(all), el);
    }).catch(function (e) {
      el.textContent = "Chart unavailable: " + e.message;
    });
  }
  if (window.uPlot) document.querySelectorAll("[data-chart-station]").forEach(chart);

  var form = document.getElementById("pause");
  if (form) form.addEventListener("submit", function (ev) {
    ev.preventDefault();
    var out = document.getElementById("pause-reply");
    var show = function (text) { if (out) out.textContent = text; };
    var headers = {};
    var token = form.elements.token;
    if (token && token.value) headers.Authorization = "Bearer " + token.value;
    var minutes = encodeURIComponent(form.elements.minutes.value);
    fetch("/api/polling/pause?minutes=" + minutes, { method: "POST", headers: headers })
      .then(function (r) {
        return r.text().then(function (text) { show((r.ok ? "" : r.status + " ") + text); });
      })
      .catch(function (e) { show("Error: " + e.message); });
  });

  if (window.EventSource) {
    var timer = null;
    var events = new EventSource("/events");
    var reload = function () {
      if (!timer) timer = setTimeout(function () { location.reload(); }, 2000);
    };
    ["sample", "grid_event", "poll_failure"].forEach(function (type) {
      events.addEventListener(type, reload);
    });
  }
})();
