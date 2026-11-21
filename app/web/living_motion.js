// ChatW Living Motion Engine (Spec 049)
// GPU-only cursor spotlight tracking (--mx / --my) & celebration micro-spark burst.
// Automatically pauses when html[data-reduce-motion="true"] or prefers-reduced-motion: reduce.
(() => {
  if (window.__cwLivingMotionInit) return;
  window.__cwLivingMotionInit = true;

  const reducedMotion = () =>
    document.documentElement.getAttribute('data-reduce-motion') === 'true' ||
    (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches);

  // 1. Delegated cursor spotlight on .cw-spotlight interactive cards
  let activeCard = null;
  let pendingX = 0;
  let pendingY = 0;
  let rafId = 0;

  const flushSpotlight = () => {
    rafId = 0;
    if (!activeCard || reducedMotion()) return;
    const rect = activeCard.getBoundingClientRect();
    const x = pendingX - rect.left;
    const y = pendingY - rect.top;
    activeCard.style.setProperty('--mx', `${x.toFixed(1)}px`);
    activeCard.style.setProperty('--my', `${y.toFixed(1)}px`);
  };

  document.addEventListener(
    'pointermove',
    (e) => {
      if (reducedMotion()) return;
      const target = e.target && e.target.closest ? e.target.closest('.cw-spotlight') : null;
      if (target !== activeCard) {
        if (activeCard) activeCard.classList.remove('cw-spot-on');
        activeCard = target;
        if (activeCard) activeCard.classList.add('cw-spot-on');
      }
      if (!activeCard) return;
      pendingX = e.clientX;
      pendingY = e.clientY;
      if (!rafId) rafId = window.requestAnimationFrame(flushSpotlight);
    },
    { passive: true },
  );

  document.addEventListener(
    'pointerleave',
    () => {
      if (activeCard) {
        activeCard.classList.remove('cw-spot-on');
        activeCard = null;
      }
    },
    { passive: true },
  );

  // 2. Celebratory GPU micro-spark burst (onboarding finish, checklist milestones)
  let lastPointer = { x: window.innerWidth / 2, y: window.innerHeight / 2 };
  document.addEventListener(
    'pointerdown',
    (e) => {
      lastPointer = { x: e.clientX, y: e.clientY };
    },
    { passive: true },
  );

  window.cwSparkBurst = (originX, originY) => {
    if (reducedMotion()) return;
    const cx = typeof originX === 'number' ? originX : lastPointer.x || window.innerWidth / 2;
    const cy = typeof originY === 'number' ? originY : lastPointer.y || window.innerHeight / 2;
    const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() || '#2764E7';
    const palette = [accent, '#38BDF8', '#10B981', '#F59E0B', '#F472B6', '#A78BFA'];

    const host = document.createElement('div');
    host.className = 'cw-spark-host';
    host.setAttribute('aria-hidden', 'true');
    host.style.cssText = `position:fixed;left:${cx}px;top:${cy}px;width:0;height:0;pointer-events:none;z-index:10050;`;

    const count = 20;
    for (let i = 0; i < count; i++) {
      const p = document.createElement('span');
      const angle = (Math.PI * 2 * i) / count + (Math.random() - 0.5) * 0.35;
      const dist = 34 + Math.random() * 54;
      const dx = Math.cos(angle) * dist;
      const dy = Math.sin(angle) * dist - 12;
      const size = 5 + Math.random() * 4.5;
      const rot = Math.round(Math.random() * 240 - 120);
      const color = palette[i % palette.length];
      p.style.cssText =
        `position:absolute;left:0;top:0;width:${size}px;height:${size}px;` +
        `border-radius:${i % 3 === 0 ? '999px' : '2.5px'};background:${color};` +
        `transform:translate(-50%,-50%) scale(0.3);opacity:1;` +
        `transition:transform 620ms cubic-bezier(0.16,1,0.3,1),opacity 620ms ease-out;will-change:transform,opacity;`;
      host.appendChild(p);
      requestAnimationFrame(() => {
        p.style.transform = `translate(calc(-50% + ${dx.toFixed(1)}px), calc(-50% + ${dy.toFixed(1)}px)) rotate(${rot}deg) scale(1)`;
        p.style.opacity = '0';
      });
    }

    document.body.appendChild(host);
    window.setTimeout(() => {
      if (host.parentNode) host.parentNode.removeChild(host);
    }, 700);
  };
})();
