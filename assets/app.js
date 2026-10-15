// bluetti-controller front end (ROADMAP.md §8.4 j). Vanilla, no build step,
// no external URL: the page must work offline in an outage.
//
// Lowest overhead first: after the page loads, ONE /api/chart request draws
// every panel, and nothing else goes over the network until the owner presses
// Refresh or picks an auto-refresh interval (paused while the tab is hidden).
// URL state like Grafana: ?from=now-12h&to=now (or ms epochs) &refresh=5m.
//
// DOM contract (status.html): #range #custom #from #to #apply #refresh #every,
// #live and #tables (swapped from a fresh GET / on refresh), #age (data-ts,
// data-now, data-stale, data-at), #c-soc #c-out_w #c-in_w chart slots, and
// the pause form (POST /api/polling/pause?minutes=N, optional Bearer token).
(function () {
  "use strict";
  var $ = function (id) { return document.getElementById(id); };
  var UNIT = { m: 60, h: 3600, d: 86400 };
  var COLORS = ["#5794f2", "#73bf69", "#ff9830", "#b877d9", "#f2495c", "#8ab8ff"];
  var PANELS = [["soc", "%"], ["out_w", "W"], ["in_w", "W"]];
  var DEFAULT = "now-12h";

  // "12h" → 43200; anything else → 0.
  function secs(s) {
    var m = /^(\d+)([mhd])$/.exec(s || "");
    return m ? m[1] * UNIT[m[2]] : 0;
  }
  function valid(from, to) {
    return (/^now-\d+[mhd]$/.test(from) || /^\d+$/.test(from)) &&
      (to === "now" || /^\d+$/.test(to));
  }

  var q = new URLSearchParams(location.search);
  var range = { from: q.get("from") || DEFAULT, to: q.get("to") || "now" };
  if (!valid(range.from, range.to)) range = { from: DEFAULT, to: "now" };
  // Only a listed interval: "30d" would overflow setInterval's 2^31 ms.
  var every = /^(1m|5m|15m|30m|1h)$/.test(q.get("refresh")) ? q.get("refresh") : "";

  // [from, to] in unix seconds; a bad absolute range falls back to 12 h.
  function win() {
    var now = Math.floor(Date.now() / 1000);
    var from = range.from.indexOf("now-") === 0 ? now - secs(range.from.slice(4)) :
      Math.floor(range.from / 1000);
    var to = range.to === "now" ? now : Math.floor(range.to / 1000);
    return from < to ? [from, to] : [now - secs("12h"), now];
  }

  function save() {
    var p = new URLSearchParams();
    if (range.from !== DEFAULT || range.to !== "now") {
      p.set("from", range.from);
      p.set("to", range.to);
    }
    if (every) p.set("refresh", every);
    var s = p.toString();
    history.replaceState(null, "", s ? "?" + s : location.pathname);
  }

  // datetime-local value ↔ ms epoch, in local time.
  function local(ms) {
    var d = new Date(ms - new Date(ms).getTimezoneOffset() * 60000);
    return d.toISOString().slice(0, 16);
  }

  function showRange() {
    var rel = range.to === "now" && $("range").querySelector('option[value="' + range.from + '"]');
    $("range").value = rel ? range.from : "custom";
    $("custom").hidden = !!rel;
    if (!rel) {
      var w = win();
      $("from").value = local(w[0] * 1000);
      $("to").value = local(w[1] * 1000);
    }
  }

  function setRange(from, to) {
    range = { from: from, to: to };
    save();
    showRange();
    load();
  }

  // --- charts -------------------------------------------------------------
  var data = null;
  var charts = [];

  function css(name) {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  }

  // A null after a gap of more than 3 steps, so the line breaks there.
  function gaps(t, v, step) {
    var T = [], V = [];
    for (var i = 0; i < t.length; i++) {
      if (i && t[i] - t[i - 1] > 3 * step) { T.push(t[i - 1] + step); V.push(null); }
      T.push(t[i]);
      V.push(v[i]);
    }
    return [T, V];
  }

  function build() {
    charts.forEach(function (c) { c.destroy(); });
    charts = [];
    if (!data || !window.uPlot) return;
    var muted = css("--muted"), grid = css("--grid"), band = css("--outage");
    var house = data.outages.filter(function (o) { return o.scope === "house"; });
    var axis = { stroke: muted, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid, width: 1 } };
    var bands = function (u) {
      var ctx = u.ctx, b = u.bbox;
      ctx.save();
      ctx.fillStyle = band;
      house.forEach(function (o) {
        var a = u.valToPos(Math.max(o.started_at, data.from), "x", true);
        var z = u.valToPos(Math.min(o.ended_at || data.to, data.to), "x", true);
        ctx.fillRect(a, b.top, Math.max(z - a, 1), b.height);
      });
      ctx.restore();
    };
    var zoom = function (u) {
      var s = u.select;
      // Synced charts replay the mouseup with no event of their own: only the
      // chart under the mouse zooms, so one drag is one request.
      if (!u.cursor.event) {
        if (s.width) u.setSelect({ left: 0, top: 0, width: 0, height: 0 }, false);
        return;
      }
      if (s.width < 5) return;
      var a = Math.floor(u.posToVal(s.left, "x")), z = Math.ceil(u.posToVal(s.left + s.width, "x"));
      u.setSelect({ left: 0, top: 0, width: 0, height: 0 }, false);
      setRange(String(a * 1000), String(z * 1000));
    };
    PANELS.forEach(function (p) {
      var el = $("c-" + p[0]);
      el.textContent = "";
      var tables = data.stations.map(function (s) { return gaps(s.t, s[p[0]], data.step); });
      // Mode 2 keeps the gap nulls across the stations' different timestamps.
      var joined = tables.length ? uPlot.join(tables, tables.map(function () { return [0, 2]; })) : [[]];
      charts.push(new uPlot({
        width: el.clientWidth || 600,
        height: 200,
        cursor: { sync: { key: "dash" }, drag: { x: true, y: false, setScale: false } },
        scales: {
          x: { time: true, range: function () { return [data.from, data.to]; } },
          y: p[1] === "%" ? { range: [0, 100] } : { range: function (u, lo, hi) { return [0, Math.max(10, hi || 0) * 1.1]; } }
        },
        series: [{}].concat(data.stations.map(function (s, i) {
          return { label: s.name, stroke: COLORS[i % COLORS.length], width: 1.5,
            value: function (u, v) { return v == null ? "—" : v.toFixed(p[1] === "%" ? 0 : 1) + " " + p[1]; } };
        })),
        axes: [axis, Object.assign({ size: 48 }, axis)],
        hooks: { drawClear: [bands], setSelect: [zoom] }
      }, joined, el));
    });
  }

  function failed(e) {
    PANELS.forEach(function (p) { $("c-" + p[0]).textContent = "Chart unavailable: " + e.message; });
  }

  // Only the newest request draws: a slow 30 d answer never overwrites 1 h.
  var seq = 0;
  function load() {
    var w = win(), my = ++seq;
    return fetch("/api/chart?from=" + w[0] + "&to=" + w[1]).then(function (r) {
      return r.json().then(function (body) {
        if (!r.ok) throw new Error(body.error || r.status);
        if (my !== seq) return;
        data = body;
        build();
      });
    }).catch(function (e) { if (my === seq) failed(e); });
  }

  // --- data age -------------------------------------------------------------
  var rendered = Date.now() / 1000;
  function span(s) {
    if (s < 90) return s + " s";
    if (s < 5400) return Math.round(s / 60) + " min";
    return Math.floor(s / 3600) + " h " + Math.round(s % 3600 / 60) + " min";
  }
  var broken = false;  // the last refresh failed: its message stays up
  function age() {
    var el = $("age"), ts = el && +el.dataset.ts;
    if (!ts || broken) return;
    // Server clock: the page's `now` plus the time since it arrived.
    var s = Math.max(0, Math.round(+el.dataset.now + Date.now() / 1000 - rendered - ts));
    var stale = s > +el.dataset.stale;
    el.className = stale ? "age stale" : "age";
    el.textContent = (stale ? "STALE — no new sample for " : "Last sample ") + span(s) +
      (stale ? " (last " : " ago (") + el.dataset.at + ")";
  }

  // --- refresh --------------------------------------------------------------
  var busy = false, timer = null, missed = false;
  function refresh() {
    if (busy) return;
    busy = true;
    $("refresh").disabled = true;
    var page = fetch(location.pathname).then(function (r) {
      if (!r.ok) throw new Error("status page " + r.status);
      return r.text();
    }).then(function (html) {
      var doc = new DOMParser().parseFromString(html, "text/html");
      ["live", "tables"].forEach(function (id) {
        var fresh = doc.getElementById(id);
        if (fresh && $(id)) $(id).replaceWith(fresh);
      });
      rendered = Date.now() / 1000;
    });
    Promise.all([page, load()]).then(function () {
      broken = false;
      age();
    }, function (e) {
      broken = true;
      // Not age(): it would overwrite the error with the old page's age.
      $("age").textContent = "Refresh failed: " + e.message + " — showing older data";
      $("age").className = "age stale";
    }).then(function () {
      busy = false;
      $("refresh").disabled = false;
    });
  }
  function schedule() {
    clearInterval(timer);
    timer = every ? setInterval(function () {
      if (document.hidden) missed = true; else refresh();
    }, secs(every) * 1000) : null;
  }

  // --- wiring ---------------------------------------------------------------
  if ($("range")) {
    showRange();
    $("every").value = every;
    $("range").onchange = function () {
      if (this.value === "custom") { $("custom").hidden = false; return; }
      setRange(this.value, "now");
    };
    $("apply").onclick = function () {
      var a = new Date($("from").value).getTime(), z = new Date($("to").value).getTime();
      if (a < z) setRange(String(a), String(z));
    };
    $("refresh").onclick = refresh;
    $("every").onchange = function () { every = this.value; save(); schedule(); };
    schedule();
    load();
    age();
    setInterval(function () { if (!document.hidden) age(); }, 30000);
    document.addEventListener("visibilitychange", function () {
      if (document.hidden) return;
      if (missed) { missed = false; refresh(); } else age();
    });
    addEventListener("themechange", build);
    var rt = null;
    addEventListener("resize", function () {
      clearTimeout(rt);
      rt = setTimeout(function () {
        charts.forEach(function (c) { c.setSize({ width: c.root.parentNode.clientWidth, height: 200 }); });
      }, 200);
    });
  }

  var form = $("pause");
  if (form) form.addEventListener("submit", function (ev) {
    ev.preventDefault();
    var out = $("pause-reply");
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
})();
