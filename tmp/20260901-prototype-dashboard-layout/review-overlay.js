/**
 * review-overlay.js — Drop-in annotation overlay for static page reviews.
 * Add <script src="review-overlay.js"></script> to any HTML page.
 *
 * Three modes via toolbar:
 *   Browse (default) — page interactive, existing annotations editable
 *   Comment tool     — block highlight on hover, click = pin + text bubble
 *   Draw tool        — freehand draw, bubble opens after each stroke for text
 */
;(function () {
  'use strict'
  if (window.__rovLoaded) return
  window.__rovLoaded = true

  const PREFIX = 'rov'
  const STORAGE_KEY = `${PREFIX}:${location.pathname}`
  const NAME_KEY = `${PREFIX}:reviewer`
  const DRAG_THRESHOLD = 4
  const UNDO_MS = 6000
  const svgNS = 'http://www.w3.org/2000/svg'

  let state = {
    comments: load('comments'),
    drawings: load('drawings'),
    nextId: 0,
    reviewer: localStorage.getItem(NAME_KEY) || '',
    drawColor: '#e74c3c',
    panelOpen: false,
    tool: 'browse', // 'browse' | 'comment' | 'draw'
  }
  state.nextId = Math.max(0,
    ...state.comments.map(c => c.id),
    ...state.drawings.map(d => d.id)
  ) + 1

  // ─── Storage ──────────────────────────────────────────────
  function load(key) {
    try { return JSON.parse(localStorage.getItem(STORAGE_KEY + (key === 'comments' ? '' : ':' + key))) || [] }
    catch { return [] }
  }
  function stripInternal(items) {
    return items.map(item => {
      const clean = {}
      for (const k of Object.keys(item)) { if (!k.startsWith('_')) clean[k] = item[k] }
      return clean
    })
  }
  function saveComments() {
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(stripInternal(state.comments))) }
    catch { toast('Storage full') }
  }
  function saveDrawings() {
    try { localStorage.setItem(STORAGE_KEY + ':drawings', JSON.stringify(stripInternal(state.drawings))) }
    catch { toast('Storage full') }
  }

  // ─── CSS ──────────────────────────────────────────────────
  const css = document.createElement('style')
  css.textContent = `
#${PREFIX}-root{position:absolute;top:0;left:0;width:100%;pointer-events:none;z-index:99990}
#${PREFIX}-root *{box-sizing:border-box}

/* ── Highlight layer (never receives events) ── */
.${PREFIX}-hl{position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none!important;z-index:100000}
.${PREFIX}-block-hl{position:absolute;border:2px solid #2a78d6;background:rgba(42,120,214,.06);border-radius:8px;opacity:0;transition:opacity .15s;pointer-events:none!important}
.${PREFIX}-block-hl.vis{opacity:1}
.${PREFIX}-block-hl-label{position:absolute;top:-22px;left:4px;background:#2a78d6;color:#fff;font:600 11px/1 system-ui,sans-serif;padding:3px 8px;border-radius:4px;white-space:nowrap;max-width:280px;overflow:hidden;text-overflow:ellipsis}
.${PREFIX}-pin-ring{position:absolute;width:40px;height:40px;border:2px solid #2a78d6;border-radius:50%;transform:translate(-50%,-50%);pointer-events:none!important;opacity:0;transition:opacity .15s}
.${PREFIX}-pin-ring.vis{opacity:1}
.${PREFIX}-stroke-glow{fill:none;stroke-linecap:round;stroke-linejoin:round;pointer-events:none!important}

/* ── Capture layer ── */
.${PREFIX}-capture{position:absolute;top:0;left:0;width:100%;height:100%;z-index:100010;pointer-events:none}
.${PREFIX}-capture.comment-active{pointer-events:auto;cursor:crosshair}
.${PREFIX}-capture.draw-active{pointer-events:auto;cursor:crosshair}

/* ── Annotation layer ── */
.${PREFIX}-annot{position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;z-index:100020}
.${PREFIX}-annot-svg{position:absolute;top:0;left:0;overflow:visible;pointer-events:none}
.${PREFIX}-stroke-vis{fill:none;stroke-linecap:round;stroke-linejoin:round;pointer-events:none}
.${PREFIX}-stroke-hit{fill:none;stroke:transparent;stroke-width:16;stroke-linecap:round;stroke-linejoin:round;pointer-events:none;cursor:pointer}
.${PREFIX}-stroke-hit.active{pointer-events:auto}

.${PREFIX}-pin{position:absolute;width:28px;height:28px;border-radius:50%;background:#e74c3c;color:#fff;font:700 13px/28px system-ui,sans-serif;text-align:center;cursor:pointer;box-shadow:0 2px 8px rgba(0,0,0,.3);user-select:none;transform:translate(-50%,-50%);pointer-events:auto;transition:transform .15s}
.${PREFIX}-pin:hover{transform:translate(-50%,-50%) scale(1.15)}
.${PREFIX}-pin.active{background:#c0392b;transform:translate(-50%,-50%) scale(1.15)}

/* ── Bubble layer ── */
.${PREFIX}-blayer{position:absolute;top:0;left:0;width:0;height:0;z-index:100030;pointer-events:none}
.${PREFIX}-bubble{position:absolute;width:280px;background:#fff;border:1px solid #ddd;border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,.15);padding:10px 12px;font:14px/1.4 system-ui,sans-serif;color:#1a1a1a;text-align:left;pointer-events:auto}
.${PREFIX}-bubble textarea{width:100%;min-height:60px;border:1px solid #ccc;border-radius:4px;padding:6px 8px;font:inherit;resize:vertical;box-sizing:border-box}
.${PREFIX}-bubble-meta{font-size:11px;color:#888;margin-bottom:6px;overflow-wrap:anywhere}
.${PREFIX}-bubble-actions{display:flex;gap:6px;margin-top:8px;justify-content:flex-end}
.${PREFIX}-bubble-actions button{font:600 12px system-ui;padding:4px 12px;border-radius:4px;cursor:pointer;border:1px solid #ccc;background:#fff;color:#333}
.${PREFIX}-bubble-actions .save{background:#2a78d6;color:#fff;border-color:#2a78d6}
.${PREFIX}-bubble-actions .del{color:#e74c3c;border-color:#e74c3c}

/* ── Panel ── */
.${PREFIX}-panel{position:fixed;top:0;right:0;width:min(380px,100vw);height:100dvh;background:#fff;border-left:1px solid #ddd;box-shadow:-4px 0 20px rgba(0,0,0,.1);z-index:100040;transform:translateX(100%);transition:transform .25s ease;display:flex;flex-direction:column;font:14px/1.4 system-ui,sans-serif;color:#1a1a1a;pointer-events:auto}
.${PREFIX}-panel.open{transform:translateX(0)}
.${PREFIX}-panel-hd{padding:14px 16px;border-bottom:1px solid #eee;display:flex;align-items:center;gap:8px}
.${PREFIX}-panel-hd-title{font-weight:600;font-size:15px;flex:1}
.${PREFIX}-panel-hd button{background:none;border:1px solid #ddd;border-radius:4px;font:600 11px system-ui;padding:5px 10px;cursor:pointer;color:#555;white-space:nowrap}
.${PREFIX}-panel-hd button:hover{background:#f5f5f5}
.${PREFIX}-panel-hd .closep{border:none;font-size:20px;color:#888;padding:0 4px;margin-left:4px}
.${PREFIX}-panel-actions{display:flex;gap:6px;padding:10px 16px;border-bottom:1px solid #eee;background:#fafafa}
.${PREFIX}-panel-actions button{flex:1;padding:7px 0;border:1px solid #ddd;border-radius:6px;font:600 12px system-ui;cursor:pointer;background:#fff;color:#555;transition:background .15s}
.${PREFIX}-panel-actions button:hover{background:#f0f0f0}
.${PREFIX}-panel-actions .clr{color:#e74c3c;border-color:#f5c6c6}
.${PREFIX}-panel-actions .clr:hover{background:#fef0f0}
.${PREFIX}-panel-body{flex:1;overflow-y:auto;padding:8px 0}
.${PREFIX}-panel-grp{margin-bottom:8px}
.${PREFIX}-panel-grp-title{font:600 12px system-ui;color:#888;text-transform:uppercase;letter-spacing:.5px;padding:8px 16px 6px;background:#fafafa;border-bottom:1px solid #f0f0f0;position:sticky;top:0;z-index:1}
.${PREFIX}-panel-item{display:flex;gap:8px;padding:8px 16px;cursor:pointer;align-items:flex-start;position:relative;transition:background .1s}
.${PREFIX}-panel-item:hover{background:#f5f7fa}
.${PREFIX}-panel-item+.${PREFIX}-panel-item{border-top:1px solid #f5f5f5}
.${PREFIX}-panel-num{width:24px;height:24px;border-radius:50%;background:#e74c3c;color:#fff;font:700 11px/24px system-ui;text-align:center;flex-shrink:0;margin-top:1px}
.${PREFIX}-panel-num.drw{background:#8e44ad;font-size:13px}
.${PREFIX}-panel-content{flex:1;min-width:0}
.${PREFIX}-panel-txt{font-size:13px;color:#333;overflow-wrap:anywhere;line-height:1.35}
.${PREFIX}-panel-who{font-size:11px;color:#aaa;margin-top:2px}
.${PREFIX}-panel-del{opacity:0;transition:opacity .15s;background:none;border:none;color:#ccc;font-size:16px;cursor:pointer;padding:2px 4px;flex-shrink:0;margin-top:1px}
.${PREFIX}-panel-del:hover{color:#e74c3c}
.${PREFIX}-panel-item:hover .${PREFIX}-panel-del{opacity:1}
.${PREFIX}-panel-empty{color:#aaa;font-style:italic;padding:32px 16px;text-align:center;line-height:1.5}

/* ── Import modal ── */
.${PREFIX}-import-modal{position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,.4);z-index:100060;display:flex;align-items:center;justify-content:center;pointer-events:auto}
.${PREFIX}-import-modal>div{background:#fff;border-radius:12px;padding:20px;width:400px;max-width:90vw;box-shadow:0 8px 32px rgba(0,0,0,.2);font:14px/1.4 system-ui,sans-serif}
.${PREFIX}-import-modal textarea{width:100%;height:160px;border:1px solid #ccc;border-radius:6px;font:12px/1.4 ui-monospace,monospace;padding:8px;margin:10px 0;resize:vertical;box-sizing:border-box}
.${PREFIX}-import-modal .actions{display:flex;gap:8px;justify-content:flex-end}
.${PREFIX}-import-modal .actions button{padding:8px 16px;border-radius:6px;font:600 13px system-ui;cursor:pointer;border:1px solid #ddd;background:#fff;color:#555}
.${PREFIX}-import-modal .actions .load{background:#2a78d6;color:#fff;border-color:#2a78d6}
.${PREFIX}-import-modal .actions .load:hover{background:#1d6bc3}

/* ── Toolbar ── */
.${PREFIX}-tb-wrap{position:fixed;bottom:1.5rem;left:1.5rem;z-index:100050;display:flex;align-items:center;pointer-events:auto}
.${PREFIX}-tb-toggle{position:relative;flex-shrink:0;width:40px;height:40px;border-radius:50%;border:none;background:#1a1a1a;color:#fff;font-size:18px;line-height:1;cursor:pointer;display:flex;align-items:center;justify-content:center;box-shadow:0 4px 14px rgba(0,0,0,.25);transition:background .15s,transform .15s;z-index:2}
.${PREFIX}-tb-toggle:hover{transform:scale(1.06)}
.${PREFIX}-tb-toggle.on{background:#2a78d6}
.${PREFIX}-tb-badge{position:absolute;top:-3px;right:-3px;min-width:16px;height:16px;padding:0 4px;border-radius:8px;background:#e74c3c;color:#fff;font:700 10px/16px system-ui,sans-serif;text-align:center;display:none}
.${PREFIX}-tb-tools{display:flex;gap:4px;align-items:center;background:#fff;border-radius:22px;box-shadow:0 4px 20px rgba(0,0,0,.18);border:1px solid rgba(0,0,0,.08);box-sizing:border-box;max-width:0;opacity:0;overflow:hidden;white-space:nowrap;margin-left:0;padding:0;transition:max-width .25s ease,opacity .2s ease,margin-left .25s ease,padding .25s ease}
.${PREFIX}-tb-tools.open,.${PREFIX}-tb-wrap:hover .${PREFIX}-tb-tools{max-width:520px;opacity:1;margin-left:8px;padding:6px 8px}
.${PREFIX}-tb-tools button{padding:8px 16px;border-radius:20px;border:none;font:600 13px system-ui,sans-serif;cursor:pointer;transition:background .15s,color .15s;background:transparent;color:#555}
.${PREFIX}-tb-tools button:hover{background:#f0f0f0}
.${PREFIX}-tb-tools button.on{background:#2a78d6;color:#fff}
.${PREFIX}-tb-tools button.on:hover{background:#1d6bc3}
.${PREFIX}-tb-sep{width:1px;height:24px;background:#ddd;margin:0 2px}
.${PREFIX}-tb-colors{display:flex;gap:3px;align-items:center;padding:0 4px}
.${PREFIX}-tb-colors button{width:20px;height:20px;border-radius:50%;border:2px solid transparent;padding:0;min-width:0;box-shadow:none}
.${PREFIX}-tb-colors button.sel{border-color:#333}
.${PREFIX}-tb-colors button:hover{transform:scale(1.15)}

/* ── Toast ── */
.${PREFIX}-toast{position:fixed;bottom:80px;left:50%;transform:translateX(-50%);background:#333;color:#fff;padding:10px 20px;border-radius:8px;font:14px system-ui;z-index:100060;opacity:0;transition:opacity .3s;display:flex;gap:12px;align-items:center;white-space:nowrap}
.${PREFIX}-toast.show{opacity:1}
.${PREFIX}-toast button{background:none;border:1px solid rgba(255,255,255,.4);color:#fff;font:600 12px system-ui;padding:2px 10px;border-radius:4px;cursor:pointer}

/* ── Name prompt ── */
.${PREFIX}-namep{position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,.4);z-index:100060;display:flex;align-items:center;justify-content:center;pointer-events:auto}
.${PREFIX}-namep>div{background:#fff;border-radius:12px;padding:24px;width:320px;box-shadow:0 8px 32px rgba(0,0,0,.2);font:15px/1.4 system-ui,sans-serif}
.${PREFIX}-namep input{width:100%;padding:8px 10px;border:1px solid #ccc;border-radius:6px;font:inherit;margin:12px 0;box-sizing:border-box}
.${PREFIX}-namep button{width:100%;padding:10px;background:#2a78d6;color:#fff;border:none;border-radius:6px;font:600 14px system-ui;cursor:pointer}
`
  document.head.appendChild(css)

  // ─── DOM layers ───────────────────────────────────────────
  const root = document.createElement('div')
  root.id = `${PREFIX}-root`
  document.documentElement.appendChild(root)

  const hlLayer = document.createElement('div')
  hlLayer.className = `${PREFIX}-hl`
  root.appendChild(hlLayer)

  const blockHl = document.createElement('div')
  blockHl.className = `${PREFIX}-block-hl`
  blockHl.innerHTML = `<div class="${PREFIX}-block-hl-label"></div>`
  hlLayer.appendChild(blockHl)
  const blockHlLabel = blockHl.firstElementChild

  const pinRing = document.createElement('div')
  pinRing.className = `${PREFIX}-pin-ring`
  hlLayer.appendChild(pinRing)

  const hlSvg = document.createElementNS(svgNS, 'svg')
  hlSvg.style.cssText = 'position:absolute;top:0;left:0;width:100%;height:100%;overflow:visible'
  hlLayer.appendChild(hlSvg)
  let glowPath = null

  const captureLayer = document.createElement('div')
  captureLayer.className = `${PREFIX}-capture`
  root.appendChild(captureLayer)

  const annotLayer = document.createElement('div')
  annotLayer.className = `${PREFIX}-annot`
  root.appendChild(annotLayer)

  const annotSvg = document.createElementNS(svgNS, 'svg')
  annotSvg.setAttribute('class', `${PREFIX}-annot-svg`)
  annotSvg.style.overflow = 'visible'
  annotLayer.appendChild(annotSvg)

  // stroke delete handled inside bubble only

  const bubbleLayer = document.createElement('div')
  bubbleLayer.className = `${PREFIX}-blayer`
  root.appendChild(bubbleLayer)

  function syncSize() {
    const w = document.documentElement.scrollWidth
    const h = document.documentElement.scrollHeight
    for (const el of [root, hlLayer, captureLayer, annotLayer]) el.style.height = h + 'px'
    for (const s of [annotSvg, hlSvg]) { s.setAttribute('width', w); s.setAttribute('height', h) }
  }
  syncSize()
  new ResizeObserver(syncSize).observe(document.documentElement)

  // ─── Tool switching ──────────────────────────────────────
  function setTool(tool) {
    closeBubble()
    clearSelection()
    hideBlockHl()
    hidePinRing()
    clearStrokeGlow()
    state.tool = tool

    captureLayer.classList.remove('comment-active', 'draw-active')
    if (tool === 'comment') captureLayer.classList.add('comment-active')
    if (tool === 'draw') captureLayer.classList.add('draw-active')

    // stroke hit-paths only active in browse (for manipulation)
    annotSvg.querySelectorAll(`.${PREFIX}-stroke-hit`).forEach(h => {
      h.classList.toggle('active', tool === 'browse')
    })

    updateToolbar()
  }

  // ─── Block detection ─────────────────────────────────────
  const BLOCK_SELECTORS = (window.ROV_BLOCK_SELECTORS || []).concat([
    '[data-screen-label]', 'section[aria-label]', '[role="region"][aria-label]',
    'section', 'article',
  ])

  function findBlock(el) {
    let node = el
    while (node && node !== document.body) {
      for (const sel of BLOCK_SELECTORS) { if (node.matches && node.matches(sel)) return node }
      node = node.parentElement
    }
    return null
  }
  function getBlockLabel(block) {
    if (!block) return null
    const sl = block.getAttribute('data-screen-label')
    if (sl) return sl
    if (block.matches('section,article,[role="region"],[role="group"]')) {
      const al = block.getAttribute('aria-label'); if (al) return al
    }
    const h = block.querySelector('h1,h2,h3,h4,h5,h6')
    return h ? h.textContent.trim() : null
  }
  function hostElementAt(x, y) {
    for (const el of document.elementsFromPoint(x, y)) {
      if (!el.closest(`#${PREFIX}-root`)) return el
    }
    return null
  }

  // ─── Screen detection ────────────────────────────────────
  function detectScreen(el) {
    if (!el) return 'General'
    let node = el
    while (node && node !== document.body) {
      if (node.getAttribute) {
        const sl = node.getAttribute('data-screen-label'); if (sl) return sl
        if (node.matches && node.matches('section,article,[role="region"],[role="group"]')) {
          const al = node.getAttribute('aria-label'); if (al) return al
        }
      }
      node = node.parentElement
    }
    node = el
    while (node && node !== document.body) {
      const h = node.querySelector && node.querySelector('h1,h2,h3,h4,h5,h6')
      if (h) return h.textContent.trim()
      node = node.parentElement
    }
    const headings = document.querySelectorAll('h1,h2,h3,h4,h5,h6,[data-screen-label]')
    let closest = null, closestDist = Infinity
    const elTop = el.getBoundingClientRect().top + window.scrollY
    for (const h of headings) {
      const hTop = h.getBoundingClientRect().top + window.scrollY
      if (hTop <= elTop && (elTop - hTop) < closestDist) { closestDist = elTop - hTop; closest = h }
    }
    if (closest) return closest.getAttribute('data-screen-label') || closest.textContent.trim()
    return 'General'
  }

  // ─── Viewport snapshot ───────────────────────────────────
  function captureViewport(bb) {
    return {
      url: location.href,
      innerWidth: window.innerWidth, innerHeight: window.innerHeight,
      scrollX: Math.round(window.scrollX), scrollY: Math.round(window.scrollY),
      devicePixelRatio: window.devicePixelRatio || 1,
      documentWidth: document.documentElement.scrollWidth,
      documentHeight: document.documentElement.scrollHeight,
      boundingBox: bb || null,
    }
  }

  // ─── Reviewer name ───────────────────────────────────────
  function ensureReviewer() {
    return new Promise((resolve, reject) => {
      if (state.reviewer) return resolve(state.reviewer)
      const ov = document.createElement('div')
      ov.className = `${PREFIX}-namep`
      ov.innerHTML = `<div>
        <div style="font-weight:600;font-size:17px">Your name</div>
        <div style="color:#666;margin-top:4px">Shown on comments so the team knows who said what.</div>
        <input type="text" placeholder="e.g. Sarah" autofocus>
        <button>Start reviewing</button></div>`
      document.body.appendChild(ov)
      const inp = ov.querySelector('input')
      const submit = () => {
        const name = inp.value.trim()
        if (!name) { inp.focus(); return }
        state.reviewer = name
        localStorage.setItem(NAME_KEY, name)
        ov.remove(); resolve(name)
      }
      const cancel = () => { ov.remove(); reject('cancelled') }
      ov.querySelector('button').addEventListener('click', submit)
      inp.addEventListener('keydown', e => { if (e.key === 'Enter') submit(); if (e.key === 'Escape') cancel() })
      ov.addEventListener('click', e => { if (e.target === ov) cancel() })
      setTimeout(() => inp.focus(), 50)
    })
  }

  // ─── Highlight: block ────────────────────────────────────
  let curHlBlock = null
  function showBlockHl(block) {
    if (block === curHlBlock) return
    curHlBlock = block
    if (!block) { blockHl.classList.remove('vis'); return }
    posBlockHl()
    const label = getBlockLabel(block)
    blockHlLabel.textContent = label || ''
    blockHlLabel.style.display = label ? '' : 'none'
    blockHl.classList.add('vis')
  }
  function posBlockHl() {
    if (!curHlBlock) return
    const r = curHlBlock.getBoundingClientRect(), pad = 4
    blockHl.style.top = (r.top + scrollY - pad) + 'px'
    blockHl.style.left = (r.left + scrollX - pad) + 'px'
    blockHl.style.width = (r.width + pad * 2) + 'px'
    blockHl.style.height = (r.height + pad * 2) + 'px'
  }
  function hideBlockHl() { curHlBlock = null; blockHl.classList.remove('vis') }

  function showPinRing(pin) {
    const r = pin.getBoundingClientRect()
    pinRing.style.top = (r.top + r.height / 2 + scrollY) + 'px'
    pinRing.style.left = (r.left + r.width / 2 + scrollX) + 'px'
    pinRing.classList.add('vis')
  }
  function hidePinRing() { pinRing.classList.remove('vis') }

  function showStrokeGlow(d) {
    clearStrokeGlow()
    glowPath = document.createElementNS(svgNS, 'path')
    glowPath.setAttribute('class', `${PREFIX}-stroke-glow`)
    glowPath.setAttribute('d', ptsPath(d.points))
    glowPath.setAttribute('stroke', d.color || '#e74c3c')
    glowPath.setAttribute('stroke-width', '8')
    glowPath.setAttribute('opacity', '0.35')
    hlSvg.appendChild(glowPath)
  }
  function clearStrokeGlow() { if (glowPath) { glowPath.remove(); glowPath = null } }

  // ─── Hover highlights (tool-aware) ───────────────────────
  document.addEventListener('pointermove', e => {
    if (drawing) return // mid-stroke, don't update highlights

    const t = e.target
    // pin hover — always (browse or any tool)
    if (t.closest(`.${PREFIX}-pin`)) {
      hideBlockHl(); clearStrokeGlow()
      showPinRing(t.closest(`.${PREFIX}-pin`))
      return
    }
    // stroke hover — browse only
    if (state.tool === 'browse' && t.closest(`.${PREFIX}-stroke-hit`)) {
      hideBlockHl(); hidePinRing()
      const id = parseInt(t.closest(`.${PREFIX}-stroke-hit`).dataset.id)
      const d = state.drawings.find(dd => dd.id === id)
      if (d) showStrokeGlow(d)
      return
    }

    hidePinRing(); clearStrokeGlow()

    // block highlight — comment tool ONLY
    if (state.tool === 'comment') {
      if (t.closest(`.${PREFIX}-tb-wrap,.${PREFIX}-panel,.${PREFIX}-bubble`)) { hideBlockHl(); return }
      const hostEl = hostElementAt(e.clientX, e.clientY)
      showBlockHl(hostEl ? findBlock(hostEl) : null)
    } else {
      hideBlockHl()
    }
  }, { passive: true })

  document.addEventListener('scroll', () => { if (curHlBlock) posBlockHl() }, { passive: true })

  // ─── Helpers ─────────────────────────────────────────────
  function dist(a, b) { return Math.hypot(a.x - b.x, a.y - b.y) }
  function ptsPath(pts) {
    if (!pts || pts.length < 2) return ''
    return 'M' + pts.map(p => p.x + ',' + p.y).join(' L')
  }
  function strokeBBox(pts) {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity
    for (const p of pts) { x0 = Math.min(x0, p.x); y0 = Math.min(y0, p.y); x1 = Math.max(x1, p.x); y1 = Math.max(y1, p.y) }
    const pad = 40
    return { x: Math.round(x0 - pad), y: Math.round(y0 - pad), width: Math.round(x1 - x0 + pad * 2), height: Math.round(y1 - y0 + pad * 2) }
  }
  function escHtml(s) { return s ? s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;') : '' }

  // ─── Pin rendering ───────────────────────────────────────
  function renderPins() {
    annotLayer.querySelectorAll(`.${PREFIX}-pin`).forEach(p => p.remove())
    for (const c of state.comments) {
      const pin = document.createElement('div')
      pin.className = `${PREFIX}-pin`
      pin.textContent = c.id
      pin.style.top = c.top + 'px'
      pin.style.left = c.left + 'px'
      pin.dataset.id = c.id
      annotLayer.appendChild(pin)
    }
  }

  // ─── Drawing rendering ───────────────────────────────────
  function renderDrawings() {
    annotSvg.querySelectorAll(`.${PREFIX}-stroke-vis,.${PREFIX}-stroke-hit`).forEach(p => p.remove())
    for (const d of state.drawings) {
      const vis = document.createElementNS(svgNS, 'path')
      vis.setAttribute('class', `${PREFIX}-stroke-vis`)
      vis.setAttribute('d', ptsPath(d.points))
      vis.setAttribute('stroke', d.color || '#e74c3c')
      vis.setAttribute('stroke-width', '3')
      annotSvg.appendChild(vis)
      const hit = document.createElementNS(svgNS, 'path')
      hit.setAttribute('class', `${PREFIX}-stroke-hit`)
      hit.setAttribute('d', ptsPath(d.points))
      hit.dataset.id = d.id
      if (state.tool === 'browse') hit.classList.add('active')
      annotSvg.appendChild(hit)
    }
  }

  // ─── Bubble ──────────────────────────────────────────────
  let activeBubble = null, activeBubbleOwner = null

  function closeBubble() {
    if (!activeBubble) return
    const ta = activeBubble.querySelector('textarea')
    const owner = activeBubbleOwner
    if (ta && owner) {
      owner.text = ta.value.trim()
      if (!owner.text && owner._kind === 'comment') {
        state.comments = state.comments.filter(c => c.id !== owner.id)
        renderPins()
      }
      if (!owner.text && owner._kind === 'drawing') {
        // keep drawing even without text — it's the visual mark that matters
      }
      if (owner._kind === 'comment') saveComments()
      else saveDrawings()
      renderPanel()
    }
    activeBubble.remove()
    activeBubble = null
    activeBubbleOwner = null
    annotLayer.querySelectorAll(`.${PREFIX}-pin.active`).forEach(p => p.classList.remove('active'))
    clearSelection()
  }

  function openBubble(item, kind, anchorTop, anchorLeft) {
    if (activeBubbleOwner && activeBubbleOwner.id === item.id && activeBubbleOwner._kind === kind) {
      closeBubble(); return
    }
    closeBubble()
    item._kind = kind

    if (kind === 'comment') {
      const pinEl = annotLayer.querySelector(`.${PREFIX}-pin[data-id="${item.id}"]`)
      if (pinEl) pinEl.classList.add('active')
    }

    const bubble = document.createElement('div')
    bubble.className = `${PREFIX}-bubble`
    bubble.style.top = (anchorTop || item.top) + 'px'
    bubble.style.left = ((anchorLeft || item.left) + 20) + 'px'

    const header = document.createElement('div')
    header.style.cssText = 'display:flex;justify-content:space-between;align-items:start'
    const meta = document.createElement('div')
    meta.className = `${PREFIX}-bubble-meta`
    meta.textContent = `${item.reviewer} · ${item.screen}`
    const closeBtn = document.createElement('button')
    closeBtn.textContent = '×'
    closeBtn.style.cssText = 'background:none;border:none;font-size:18px;cursor:pointer;color:#888;padding:0 0 0 8px;line-height:1'
    closeBtn.addEventListener('click', () => closeBubble())
    header.appendChild(meta)
    header.appendChild(closeBtn)
    bubble.appendChild(header)

    const ta = document.createElement('textarea')
    ta.value = item.text || ''
    ta.placeholder = kind === 'drawing' ? 'Describe what this drawing highlights...' : 'Your comment...'
    bubble.appendChild(ta)

    const actions = document.createElement('div')
    actions.className = `${PREFIX}-bubble-actions`

    const delBtn = document.createElement('button')
    delBtn.className = 'del'
    delBtn.textContent = 'Delete'
    delBtn.addEventListener('click', () => {
      const removed = item
      activeBubbleOwner = null
      activeBubble.remove()
      activeBubble = null
      if (kind === 'comment') deleteComment(removed)
      else deleteDrawing(removed)
    })

    const saveBtn = document.createElement('button')
    saveBtn.className = 'save'
    saveBtn.textContent = 'Save'
    saveBtn.addEventListener('click', () => {
      item.text = ta.value.trim()
      if (kind === 'comment') saveComments(); else saveDrawings()
      closeBubble()
      renderPanel()
    })

    actions.appendChild(delBtn)
    actions.appendChild(saveBtn)
    bubble.appendChild(actions)

    bubble.addEventListener('pointerdown', e => e.stopPropagation())
    bubble.addEventListener('click', e => e.stopPropagation())
    ta.addEventListener('keydown', e => {
      if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
        item.text = ta.value.trim()
        if (kind === 'comment') saveComments(); else saveDrawings()
        closeBubble(); renderPanel()
      }
      if (e.key === 'Escape') { closeBubble(); e.stopPropagation() }
    })

    bubbleLayer.appendChild(bubble)
    activeBubble = bubble
    activeBubbleOwner = item

    // flip if off-screen right
    const rect = bubble.getBoundingClientRect()
    const panelW = state.panelOpen ? 360 : 0
    if (rect.right > window.innerWidth - panelW) {
      bubble.style.left = ((anchorLeft || item.left) - 300) + 'px'
    }

    ta.focus()
  }

  // ─── Undo ────────────────────────────────────────────────
  function deleteComment(c) {
    state.comments = state.comments.filter(cc => cc.id !== c.id)
    saveComments(); renderPins(); renderPanel()
    showUndo('comment', c)
  }
  function deleteDrawing(d) {
    state.drawings = state.drawings.filter(dd => dd.id !== d.id)
    saveDrawings(); renderDrawings(); renderPanel()
    clearStrokeGlow(); hideStrokeDelBtn()
    showUndo('drawing', d)
  }
  function showUndo(kind, item) {
    toastWithUndo('Deleted', () => {
      if (kind === 'comment') { state.comments.push(item); saveComments(); renderPins() }
      else { state.drawings.push(item); saveDrawings(); renderDrawings() }
      renderPanel()
    })
  }

  // ─── Stroke selection (browse mode) ──────────────────────
  let selectedStroke = null
  function selectStroke(d) { selectedStroke = d; showStrokeGlow(d) }
  function clearSelection() { selectedStroke = null; clearStrokeGlow() }

  // ─── Comment tool: capture layer pointerup = pin ─────────
  // Use pointerup (not pointerdown) so the bubble opens AFTER the click event,
  // avoiding the race where click-away immediately kills it.
  let suppressClickAway = false

  captureLayer.addEventListener('click', e => {
    if (state.tool !== 'browse') {
      e.stopPropagation()
      e.preventDefault()
    }
  })

  captureLayer.addEventListener('pointerup', e => {
    if (state.tool === 'comment') {
      e.preventDefault()
      suppressClickAway = true
      setTimeout(() => { suppressClickAway = false }, 100)
      const hostEl = hostElementAt(e.clientX, e.clientY)
      const screen = detectScreen(hostEl)
      const comment = {
        id: state.nextId++,
        top: e.pageY, left: e.pageX, screen,
        text: '', reviewer: state.reviewer,
        viewport: captureViewport({ x: e.pageX - 150, y: e.pageY - 100, width: 300, height: 200 }),
      }
      state.comments.push(comment)
      saveComments()
      renderPins()
      renderPanel()
      openBubble(comment, 'comment')
    }
  })

  // ─── Draw tool: capture layer drag = stroke ──────────────
  let drawing = null // { pts, livePath, screen }

  captureLayer.addEventListener('pointerdown', e => {
    if (state.tool !== 'draw') return
    e.preventDefault()
    captureLayer.setPointerCapture(e.pointerId)
    const hostEl = hostElementAt(e.clientX, e.clientY)
    const screen = detectScreen(hostEl)
    const pts = [{ x: e.pageX, y: e.pageY }]
    const livePath = document.createElementNS(svgNS, 'path')
    livePath.setAttribute('fill', 'none')
    livePath.setAttribute('stroke', state.drawColor)
    livePath.setAttribute('stroke-width', '3')
    livePath.setAttribute('stroke-linecap', 'round')
    livePath.setAttribute('stroke-linejoin', 'round')
    livePath.setAttribute('d', `M${e.pageX},${e.pageY}`)
    annotSvg.appendChild(livePath)
    drawing = { pts, livePath, screen, pointerId: e.pointerId }
  })

  captureLayer.addEventListener('pointermove', e => {
    if (!drawing) return
    drawing.pts.push({ x: e.pageX, y: e.pageY })
    drawing.livePath.setAttribute('d', ptsPath(drawing.pts))
  })

  captureLayer.addEventListener('pointerup', e => {
    if (!drawing) return
    captureLayer.releasePointerCapture(e.pointerId)
    drawing.livePath.remove()
    if (drawing.pts.length < 3) { drawing = null; return }
    const bb = strokeBBox(drawing.pts)
    const d = {
      id: state.nextId++,
      points: drawing.pts,
      color: state.drawColor,
      screen: drawing.screen,
      text: '',
      reviewer: state.reviewer,
      viewport: captureViewport(bb),
    }
    state.drawings.push(d)
    saveDrawings()
    renderDrawings()
    renderPanel()
    // open bubble at stroke midpoint for text
    const mid = drawing.pts[Math.floor(drawing.pts.length / 2)]
    drawing = null
    requestAnimationFrame(() => openBubble(d, 'drawing', mid.y, mid.x))
  })

  captureLayer.addEventListener('pointercancel', e => {
    if (!drawing) return
    drawing.livePath.remove()
    captureLayer.releasePointerCapture(e.pointerId)
    drawing = null
  })

  // ─── Pin interactions (always active) ────────────────────
  annotLayer.addEventListener('pointerdown', e => {
    const pinEl = e.target.closest(`.${PREFIX}-pin`)
    if (!pinEl) return
    e.stopPropagation(); e.preventDefault()
    const id = parseInt(pinEl.dataset.id)
    const comment = state.comments.find(c => c.id === id)
    if (!comment) return

    const origin = { x: e.pageX, y: e.pageY }
    let dragging = false

    const onMove = ev => {
      if (!dragging && dist(origin, { x: ev.pageX, y: ev.pageY }) > DRAG_THRESHOLD) {
        dragging = { _last: origin }
        closeBubble()
      }
      if (dragging) {
        comment.top += ev.pageY - dragging._last.y
        comment.left += ev.pageX - dragging._last.x
        pinEl.style.top = comment.top + 'px'
        pinEl.style.left = comment.left + 'px'
        dragging._last = { x: ev.pageX, y: ev.pageY }
      }
    }
    const onUp = ev => {
      pinEl.releasePointerCapture(ev.pointerId)
      document.removeEventListener('pointermove', onMove)
      document.removeEventListener('pointerup', onUp)
      if (dragging) {
        comment.viewport = captureViewport({ x: comment.left - 150, y: comment.top - 100, width: 300, height: 200 })
        saveComments()
      } else {
        openBubble(comment, 'comment')
      }
    }
    pinEl.setPointerCapture(e.pointerId)
    document.addEventListener('pointermove', onMove)
    document.addEventListener('pointerup', onUp)
  })

  // ─── Stroke interactions (browse mode) ───────────────────
  annotSvg.addEventListener('click', e => {
    if (e.target.closest(`.${PREFIX}-stroke-hit`)) e.stopPropagation()
  })
  annotSvg.addEventListener('pointerdown', e => {
    const hitEl = e.target.closest(`.${PREFIX}-stroke-hit`)
    if (!hitEl || state.tool !== 'browse') return
    e.stopPropagation(); e.preventDefault()
    const id = parseInt(hitEl.dataset.id)
    const d = state.drawings.find(dd => dd.id === id)
    if (!d) return

    const origin = { x: e.pageX, y: e.pageY }
    let dragging = false

    const onMove = ev => {
      if (!dragging && dist(origin, { x: ev.pageX, y: ev.pageY }) > DRAG_THRESHOLD) {
        dragging = { _last: origin }
        clearSelection()
      }
      if (dragging) {
        const dx = ev.pageX - dragging._last.x, dy = ev.pageY - dragging._last.y
        for (const p of d.points) { p.x += dx; p.y += dy }
        dragging._last = { x: ev.pageX, y: ev.pageY }
        renderDrawings()
      }
    }
    const onUp = ev => {
      document.removeEventListener('pointermove', onMove)
      document.removeEventListener('pointerup', onUp)
      if (dragging) {
        d.viewport = captureViewport(strokeBBox(d.points))
        saveDrawings()
      } else {
        if (selectedStroke && selectedStroke.id === d.id) { clearSelection(); return }
        selectStroke(d)
        const mid = d.points[Math.floor(d.points.length / 2)]
        openBubble(d, 'drawing', mid.y, mid.x)
      }
    }
    document.addEventListener('pointermove', onMove)
    document.addEventListener('pointerup', onUp)
  })

  // ─── Click-away closes bubble ────────────────────────────
  document.addEventListener('click', e => {
    if (suppressClickAway) return
    if (activeBubble && !e.target.closest(`.${PREFIX}-bubble,.${PREFIX}-pin,.${PREFIX}-stroke-hit`)) closeBubble()
  })

  // ─── Panel ───────────────────────────────────────────────
  const panel = document.createElement('div')
  panel.className = `${PREFIX}-panel`
  panel.innerHTML = `
    <div class="${PREFIX}-panel-hd">
      <div class="${PREFIX}-panel-hd-title">Annotations (0)</div>
      <button class="closep">&times;</button>
    </div>
    <div class="${PREFIX}-panel-actions">
      <button class="exp">Export</button>
      <button class="imp">Import</button>
      <button class="clr">Clear all</button>
    </div>
    <div class="${PREFIX}-panel-body"></div>`
  panel.querySelector('.closep').addEventListener('click', togglePanel)
  panel.querySelector('.exp').addEventListener('click', exportJson)
  panel.querySelector('.imp').addEventListener('click', showImportModal)
  panel.querySelector('.clr').addEventListener('click', clearAll)
  document.body.appendChild(panel)

  function togglePanel() {
    state.panelOpen = !state.panelOpen
    panel.classList.toggle('open', state.panelOpen)
    if (state.panelOpen) renderPanel()
    updateToolbar()
  }

  function renderPanel() {
    const body = panel.querySelector(`.${PREFIX}-panel-body`)
    const all = [
      ...state.comments.map(c => ({ ...c, kind: 'comment' })),
      ...state.drawings.map(d => ({ ...d, kind: 'drawing' })),
    ]
    if (!all.length) {
      body.innerHTML = `<div class="${PREFIX}-panel-empty">No annotations yet.<br>Use 📌 Comment or ✏ Draw to start.</div>`
      updatePanelCount(); return
    }
    const grouped = {}
    for (const item of all) { const k = item.screen || 'General'; (grouped[k] ||= []).push(item) }

    body.innerHTML = ''
    for (const [screen, items] of Object.entries(grouped)) {
      const grp = document.createElement('div')
      grp.className = `${PREFIX}-panel-grp`
      const title = document.createElement('div')
      title.className = `${PREFIX}-panel-grp-title`
      title.textContent = screen
      grp.appendChild(title)

      for (const item of items) {
        const row = document.createElement('div')
        row.className = `${PREFIX}-panel-item`

        const isDraw = item.kind === 'drawing'
        const num = document.createElement('div')
        num.className = `${PREFIX}-panel-num` + (isDraw ? ' drw' : '')
        num.textContent = isDraw ? '✎' : item.id
        row.appendChild(num)

        const content = document.createElement('div')
        content.className = `${PREFIX}-panel-content`
        const txt = document.createElement('div')
        txt.className = `${PREFIX}-panel-txt`
        if (isDraw) {
          txt.textContent = item.text || 'Drawing annotation'
        } else {
          txt.textContent = item.text || 'Empty comment'
          if (!item.text) txt.style.color = '#aaa'
        }
        const who = document.createElement('div')
        who.className = `${PREFIX}-panel-who`
        who.textContent = item.reviewer
        content.appendChild(txt)
        content.appendChild(who)
        row.appendChild(content)

        const del = document.createElement('button')
        del.className = `${PREFIX}-panel-del`
        del.textContent = '×'
        del.title = 'Delete'
        del.addEventListener('click', e => {
          e.stopPropagation()
          if (item.kind === 'comment') deleteComment(item)
          else deleteDrawing(item)
        })
        row.appendChild(del)

        row.addEventListener('click', () => {
          const id = item.id
          if (item.kind === 'comment') {
            const pin = annotLayer.querySelector(`.${PREFIX}-pin[data-id="${id}"]`)
            if (pin) {
              pin.scrollIntoView({ behavior: 'smooth', block: 'center' })
              const c = state.comments.find(cc => cc.id === id)
              if (c) setTimeout(() => openBubble(c, 'comment'), 400)
            }
          } else {
            const hit = annotSvg.querySelector(`.${PREFIX}-stroke-hit[data-id="${id}"]`)
            if (hit) {
              hit.scrollIntoView({ behavior: 'smooth', block: 'center' })
              const d = state.drawings.find(dd => dd.id === id)
              if (d) setTimeout(() => {
                const mid = d.points[Math.floor(d.points.length / 2)]
                openBubble(d, 'drawing', mid.y, mid.x)
              }, 400)
            }
          }
        })

        grp.appendChild(row)
      }
      body.appendChild(grp)
    }
    updatePanelCount()
  }

  function updatePanelCount() {
    const n = state.comments.length + state.drawings.length
    panel.querySelector(`.${PREFIX}-panel-hd-title`).textContent = `Annotations (${n})`
    updateToolbar()
  }

  // ─── Export / Import / Clear ──────────────────────────────
  function exportJson() {
    const n = state.comments.length + state.drawings.length
    if (!n) { toast('Nothing to export'); return }
    const data = {
      url: location.href,
      exportedAt: new Date().toISOString(),
      exportedBy: state.reviewer,
      comments: stripInternal(state.comments),
      drawings: stripInternal(state.drawings),
    }
    copyText(JSON.stringify(data, null, 2), `${n} annotations copied to clipboard`)
  }

  function showImportModal() {
    const modal = document.createElement('div')
    modal.className = `${PREFIX}-import-modal`
    modal.innerHTML = `<div>
      <div style="font-weight:600;font-size:16px;margin-bottom:4px">Import annotations</div>
      <div style="color:#666;font-size:13px">Paste the JSON that was copied with Export.</div>
      <textarea placeholder="Paste JSON here..."></textarea>
      <div class="actions">
        <button class="cancel">Cancel</button>
        <button class="load">Load annotations</button>
      </div>
    </div>`
    document.body.appendChild(modal)
    const ta = modal.querySelector('textarea')
    const cancel = () => modal.remove()
    modal.querySelector('.cancel').addEventListener('click', cancel)
    modal.addEventListener('click', e => { if (e.target === modal) cancel() })
    ta.addEventListener('keydown', e => { if (e.key === 'Escape') cancel() })
    modal.querySelector('.load').addEventListener('click', () => {
      const text = ta.value.trim()
      if (!text) { ta.focus(); return }
      try {
        const data = JSON.parse(text)
        modal.remove()
        doImport(data)
      } catch { ta.style.borderColor = '#e74c3c'; toast('Invalid JSON — check the pasted text') }
    })
    setTimeout(() => ta.focus(), 50)
  }

  function doImport(data) {
    const raw = {
      comments: Array.isArray(data.comments) ? data.comments : [],
      drawings: Array.isArray(data.drawings) ? data.drawings : [],
    }
    if (!raw.comments.length && !raw.drawings.length) {
      toast('No annotations found'); return
    }
    if (data.url) {
      try {
        if (new URL(data.url).pathname !== location.pathname) {
          if (!confirm(`This export is from a different page. Import anyway?`)) return
        }
      } catch { /* invalid url, ignore */ }
    }
    const source = String(data.exportedBy || 'imported')
    let pinCount = 0, drawCount = 0, skipped = 0
    for (const c of raw.comments) {
      if (typeof c.top !== 'number' || !isFinite(c.top) ||
          typeof c.left !== 'number' || !isFinite(c.left)) { skipped++; continue }
      state.comments.push({
        id: state.nextId++, top: c.top, left: c.left,
        screen: String(c.screen || 'General'),
        text: String(c.text || ''),
        reviewer: String(c.reviewer || source),
        viewport: c.viewport || null,
      })
      pinCount++
    }
    for (const d of raw.drawings) {
      if (!Array.isArray(d.points) || d.points.length < 2) { skipped++; continue }
      if (!d.points.every(p => typeof p.x === 'number' && typeof p.y === 'number' && isFinite(p.x) && isFinite(p.y))) { skipped++; continue }
      state.drawings.push({
        id: state.nextId++,
        points: d.points.map(p => ({ x: p.x, y: p.y })),
        color: String(d.color || '#e74c3c'),
        screen: String(d.screen || 'General'),
        text: String(d.text || ''),
        reviewer: String(d.reviewer || source),
        viewport: d.viewport || null,
      })
      drawCount++
    }
    saveComments(); saveDrawings()
    renderPins(); renderDrawings(); renderPanel()
    let msg = `Imported ${pinCount} comments, ${drawCount} drawings`
    if (skipped) msg += ` (${skipped} skipped)`
    toast(msg)
  }

  function clearAll() {
    const total = state.comments.length + state.drawings.length
    if (!total) { toast('Nothing to clear'); return }
    const oldComments = [...state.comments]
    const oldDrawings = [...state.drawings]
    state.comments = []; state.drawings = []
    saveComments(); saveDrawings()
    closeBubble(); clearSelection()
    renderPins(); renderDrawings(); renderPanel()
    toastWithUndo(`Cleared ${total} annotations`, () => {
      state.comments = oldComments; state.drawings = oldDrawings
      saveComments(); saveDrawings()
      renderPins(); renderDrawings(); renderPanel()
    })
  }
  function copyText(text, successMsg) {
    const msg = successMsg || 'Copied to clipboard'
    if (navigator.clipboard && window.isSecureContext) {
      navigator.clipboard.writeText(text).then(() => toast(msg), () => fbCopy(text, msg))
    } else fbCopy(text, msg)
  }
  function fbCopy(text, msg) {
    const ta = document.createElement('textarea'); ta.value = text; ta.style.cssText = 'position:fixed;left:-9999px'
    document.body.appendChild(ta); ta.select(); const ok = document.execCommand('copy'); ta.remove()
    toast(ok ? msg : 'Copy failed — select and copy manually')
  }

  // ─── Toolbar ─────────────────────────────────────────────
  const COLORS = ['#e74c3c', '#2a78d6', '#27ae60', '#f39c12']

  const tbWrap = document.createElement('div')
  tbWrap.className = `${PREFIX}-tb-wrap`

  const tbToggle = document.createElement('button')
  tbToggle.className = `${PREFIX}-tb-toggle`
  tbToggle.title = 'Review tools'
  tbToggle.textContent = '✎'
  const tbBadge = document.createElement('span')
  tbBadge.className = `${PREFIX}-tb-badge`
  tbToggle.appendChild(tbBadge)
  tbToggle.addEventListener('click', () => { tbTools.classList.toggle('open') })

  const tbTools = document.createElement('div')
  tbTools.className = `${PREFIX}-tb-tools`

  const tbComment = mk('button', '📌 Comment')
  tbComment.addEventListener('click', async () => {
    if (state.tool === 'comment') { setTool('browse'); return }
    try { await ensureReviewer() } catch { return }
    setTool('comment')
  })

  const tbSep1 = mk('div'); tbSep1.className = `${PREFIX}-tb-sep`

  const tbDraw = mk('button', '✏ Draw')
  tbDraw.addEventListener('click', async () => {
    if (state.tool === 'draw') { setTool('browse'); return }
    try { await ensureReviewer() } catch { return }
    setTool('draw')
  })

  const tbColors = document.createElement('div')
  tbColors.className = `${PREFIX}-tb-colors`
  for (const color of COLORS) {
    const btn = document.createElement('button')
    btn.style.background = color
    if (color === state.drawColor) btn.classList.add('sel')
    btn.addEventListener('click', () => {
      state.drawColor = color
      tbColors.querySelectorAll('button').forEach(b => b.classList.remove('sel'))
      btn.classList.add('sel')
    })
    tbColors.appendChild(btn)
  }

  const tbSep2 = mk('div'); tbSep2.className = `${PREFIX}-tb-sep`

  const tbPanel = mk('button', '☰')
  tbPanel.title = 'Annotation list'
  tbPanel.addEventListener('click', togglePanel)

  tbTools.appendChild(tbComment)
  tbTools.appendChild(tbSep1)
  tbTools.appendChild(tbDraw)
  tbTools.appendChild(tbColors)
  tbTools.appendChild(tbSep2)
  tbTools.appendChild(tbPanel)

  tbWrap.appendChild(tbToggle)
  tbWrap.appendChild(tbTools)
  document.body.appendChild(tbWrap)
  tbTools.classList.add('open')
  setTimeout(() => tbTools.classList.remove('open'), 1800)

  function mk(tag, text) { const el = document.createElement(tag); if (text) el.textContent = text; return el }

  function updateToolbar() {
    tbComment.classList.toggle('on', state.tool === 'comment')
    tbDraw.classList.toggle('on', state.tool === 'draw')
    tbColors.style.display = state.tool === 'draw' ? 'flex' : 'none'
    const n = state.comments.length + state.drawings.length
    tbBadge.textContent = n
    tbBadge.style.display = n ? 'block' : 'none'
    tbToggle.classList.toggle('on', state.tool !== 'browse')
  }

  // ─── Toast ───────────────────────────────────────────────
  const toastEl = document.createElement('div')
  toastEl.className = `${PREFIX}-toast`
  document.body.appendChild(toastEl)
  let toastTimer

  function toast(msg) {
    toastEl.textContent = msg
    toastEl.classList.add('show')
    clearTimeout(toastTimer)
    toastTimer = setTimeout(() => toastEl.classList.remove('show'), 2000)
  }
  function toastWithUndo(msg, fn) {
    toastEl.textContent = ''
    toastEl.appendChild(document.createTextNode(msg + ' '))
    const btn = document.createElement('button'); btn.textContent = 'Undo'
    btn.addEventListener('click', () => { fn(); toastEl.classList.remove('show'); clearTimeout(toastTimer) })
    toastEl.appendChild(btn)
    toastEl.classList.add('show')
    clearTimeout(toastTimer)
    toastTimer = setTimeout(() => toastEl.classList.remove('show'), UNDO_MS)
  }

  // ─── Keyboard ────────────────────────────────────────────
  document.addEventListener('keydown', e => {
    if (e.target.matches('input,textarea,[contenteditable]')) return
    if (e.key === 'Escape') {
      if (drawing) { drawing.livePath.remove(); captureLayer.releasePointerCapture(drawing.pointerId); drawing = null; return }
      if (selectedStroke) { clearSelection(); return }
      if (activeBubble) { closeBubble(); return }
      if (state.tool !== 'browse') { setTool('browse'); return }
      if (state.panelOpen) { togglePanel(); return }
    }
    if ((e.key === 'Delete' || e.key === 'Backspace') && selectedStroke) {
      deleteDrawing(selectedStroke); clearSelection()
    }
  })

  // ─── Init ────────────────────────────────────────────────
  try { renderPins() } catch (e) {
    console.error('[review-overlay] failed to render pins, clearing:', e)
    state.comments = []; saveComments()
  }
  try { renderDrawings() } catch (e) {
    console.error('[review-overlay] failed to render drawings, clearing:', e)
    state.drawings = []; saveDrawings()
  }
  updateToolbar()
  console.log(`[review-overlay] ${state.comments.length} pins, ${state.drawings.length} drawings. Use toolbar to annotate.`)
})()
