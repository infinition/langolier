

function mountLangolier(el, options) {
  if (!el) return null;
  var opts = options || {};
  // Everything the host may need to stop: listeners, the frame loop, and the
  // steady feed that runs while the application is ingesting.
  var teardown = [];
  var running = true;
  var feedTimer = null;
  function on(target, type, fn) {
    target.addEventListener(type, fn);
    teardown.push(function () { target.removeEventListener(type, fn); });
  }

  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reducedMotion) return;

  let pointerX = 0;
  let pointerY = 0;
  let targetPointerX = 0;
  let targetPointerY = 0;

  let idleX = 0;
  let idleY = 0;
  let idleR = 0;
  let targetIdleX = 0;
  let targetIdleY = 0;
  let targetIdleR = 0;

  let hover = 0;
  let targetHover = 0;

  let lastTime = performance.now();
  let nextIdleChange = 0;
  let reactionTimer = null;

  // Une respiration un peu organique : deux sinusoïdes superposées.
  // Ça évite l'effet mécanique "scale up / scale down".
  function breathing(t) {
    const slow = Math.sin(t * 0.00165);
    const micro = Math.sin(t * 0.0047 + 1.6) * 0.16;
    return (slow + micro) * 0.5 + 0.5;
  }

  function heartbeat(t) {
    // Très subtil. Pas un vrai battement visible, plutôt une micro-variation
    // de présence dans les yeux et la luminance.
    const x = (t % 4200) / 4200;
    const beat1 = Math.exp(-Math.pow((x - 0.08) / 0.022, 2));
    const beat2 = Math.exp(-Math.pow((x - 0.145) / 0.028, 2)) * 0.58;
    return Math.min(1, beat1 + beat2);
  }

  function chooseNewIdleTarget(now) {
    targetIdleX = (Math.random() - 0.5) * 7;
    targetIdleY = (Math.random() - 0.5) * 5;
    targetIdleR = (Math.random() - 0.5) * 1.25;
    nextIdleChange = now + 1800 + Math.random() * 2600;
  }

  function frame(now) {
    if (!running) return;
    const dt = Math.min(32, now - lastTime) / 16.67;
    lastTime = now;

    if (now >= nextIdleChange) chooseNewIdleTarget(now);

    // Interpolation douce vers la souris et les micro-mouvements aléatoires.
    const pointerEase = 1 - Math.pow(0.86, dt);
    const idleEase = 1 - Math.pow(0.975, dt);
    const hoverEase = 1 - Math.pow(0.90, dt);

    pointerX += (targetPointerX - pointerX) * pointerEase;
    pointerY += (targetPointerY - pointerY) * pointerEase;

    idleX += (targetIdleX - idleX) * idleEase;
    idleY += (targetIdleY - idleY) * idleEase;
    idleR += (targetIdleR - idleR) * idleEase;

    hover += (targetHover - hover) * hoverEase;

    const breath = breathing(now);
    const pulse = heartbeat(now);

    el.style.setProperty("--pointer-x", pointerX.toFixed(4));
    el.style.setProperty("--pointer-y", pointerY.toFixed(4));
    el.style.setProperty("--idle-x", `${idleX.toFixed(2)}px`);
    el.style.setProperty("--idle-y", `${idleY.toFixed(2)}px`);
    el.style.setProperty("--idle-r", `${idleR.toFixed(2)}deg`);
    el.style.setProperty("--breath", breath.toFixed(4));
    el.style.setProperty("--pulse", pulse.toFixed(4));
    el.style.setProperty("--hover", hover.toFixed(4));

    requestAnimationFrame(frame);
  }

  function updatePointer(clientX, clientY) {
    const r = el.getBoundingClientRect();
    const cx = r.left + r.width / 2;
    const cy = r.top + r.height / 2;

    // Valeur normalisée approximativement entre -1 et 1.
    targetPointerX = Math.max(-1, Math.min(1, (clientX - cx) / (r.width * 0.52)));
    targetPointerY = Math.max(-1, Math.min(1, (clientY - cy) / (r.height * 0.52)));
  }

  function neutralPointer() {
    targetPointerX = 0;
    targetPointerY = 0;
  }

  function react() {
    clearTimeout(reactionTimer);
    el.classList.remove("is-reacting");
    // Force le navigateur à redémarrer l'animation.
    void el.offsetWidth;
    el.classList.add("is-reacting");
    reactionTimer = setTimeout(() => {
      el.classList.remove("is-reacting");
    }, 700);
  }



  // ---------------------------------------------------------
  // Ingestion de documents
  // ---------------------------------------------------------
  const knowledgeLayer = document.querySelector("#knowledgeLayer");
  const reducedMotionForFeed = matchMedia("(prefers-reduced-motion: reduce)").matches;

  function mouthPoint() {
    const r = knowledgeLayer.getBoundingClientRect();

    // Le layer déborde volontairement de 16% autour du Langolier.
    // Ces coordonnées replacent la cible au centre réel de la bouche.
    return {
      x: r.width * 0.505,
      y: r.height * 0.515,
      width: r.width,
      height: r.height
    };
  }

  function spawnKnowledgeBit(originX, originY, delay = 0) {
    if (!knowledgeLayer || reducedMotionForFeed) return;

    const target = mouthPoint();
    const bit = document.createElement("span");
    bit.className = "knowledge-bit";
    bit.style.left = `${originX}px`;
    bit.style.top = `${originY}px`;
    knowledgeLayer.appendChild(bit);

    const bend = (Math.random() - .5) * target.width * .11;
    const midX = originX + (target.x - originX) * .58 + bend;
    const midY = originY + (target.y - originY) * .54 + (Math.random() - .5) * target.height * .08;

    const anim = bit.animate([
      {
        transform: "translate(-50%, -50%) scale(.25)",
        opacity: 0
      },
      {
        transform: `translate(${midX - originX}px, ${midY - originY}px) rotate(170deg) scale(1)`,
        opacity: .95,
        offset: .38
      },
      {
        transform: `translate(${target.x - originX}px, ${target.y - originY}px) rotate(430deg) scale(.04)`,
        opacity: 0
      }
    ], {
      duration: 620 + Math.random() * 450,
      delay,
      easing: "cubic-bezier(.18,.72,.18,1)",
      fill: "forwards"
    });

    anim.finished.finally(() => bit.remove());
  }

  function spawnDocument(index = 0, total = 10) {
    if (!knowledgeLayer || reducedMotionForFeed) return;

    const target = mouthPoint();
    const doc = document.createElement("span");
    doc.className = "flying-doc";

    // Départ depuis un anneau autour du personnage plutôt que toujours
    // depuis le même bord. Cela donne une sensation de "suction".
    const angle =
      (index / Math.max(1, total)) * Math.PI * 2 +
      (Math.random() - .5) * .75;

    const rx = target.width * (.42 + Math.random() * .16);
    const ry = target.height * (.38 + Math.random() * .15);

    let sx = target.x + Math.cos(angle) * rx;
    let sy = target.y + Math.sin(angle) * ry;

    // Un peu plus de chaos : quelques feuilles démarrent hors cadre.
    if (Math.random() < .32) {
      sx += Math.cos(angle) * target.width * .16;
      sy += Math.sin(angle) * target.height * .16;
    }

    doc.style.left = `${sx}px`;
    doc.style.top = `${sy}px`;
    knowledgeLayer.appendChild(doc);

    const side = Math.random() > .5 ? 1 : -1;
    const c1x =
      sx + (target.x - sx) * .30 +
      side * target.width * (.08 + Math.random() * .11);
    const c1y =
      sy + (target.y - sy) * .28 +
      (Math.random() - .5) * target.height * .13;

    const c2x =
      sx + (target.x - sx) * .72 -
      side * target.width * (.025 + Math.random() * .07);
    const c2y =
      sy + (target.y - sy) * .73 +
      (Math.random() - .5) * target.height * .06;

    const rot = (Math.random() - .5) * 150;
    const startScale = .62 + Math.random() * .46;
    const duration = 1080 + Math.random() * 700;
    const delay = index * 58 + Math.random() * 90;

    // Avant que la feuille ne disparaisse dans la bouche, elle se fragmente
    // en quelques pixels lumineux.
    const fragmentTimer = setTimeout(() => {
      const bx = c2x + (Math.random() - .5) * 12;
      const by = c2y + (Math.random() - .5) * 12;
      for (let i = 0; i < 3; i++) {
        spawnKnowledgeBit(bx, by, i * 34);
      }
    }, delay + duration * .67);

    const anim = doc.animate([
      {
        transform: `translate(-50%, -50%) rotate(${rot * -.18}deg) scale(${startScale})`,
        opacity: 0,
        filter: "blur(1.1px)"
      },
      {
        transform: `translate(${c1x - sx}px, ${c1y - sy}px) rotate(${rot}deg) scale(1)`,
        opacity: 1,
        filter: "blur(0px)",
        offset: .27
      },
      {
        transform: `translate(${c2x - sx}px, ${c2y - sy}px) rotate(${rot * 1.75}deg) scale(.72)`,
        opacity: .96,
        filter: "blur(.15px)",
        offset: .70
      },
      {
        transform: `translate(${target.x - sx}px, ${target.y - sy}px) rotate(${rot * 3.2}deg) scale(.035)`,
        opacity: 0,
        filter: "blur(2.4px)"
      }
    ], {
      duration,
      delay,
      easing: "cubic-bezier(.18,.72,.16,1)",
      fill: "forwards"
    });

    anim.finished.finally(() => {
      clearTimeout(fragmentTimer);
      doc.remove();
    });
  }

  function feedDocuments(amount = 11) {
    react();

    if (reducedMotionForFeed || !knowledgeLayer) return;

    el.classList.remove("is-feeding");
    void el.offsetWidth;
    el.classList.add("is-feeding");

    const count = Math.max(1, Math.min(30, amount));
    for (let i = 0; i < count; i++) {
      spawnDocument(i, count);
    }

    setTimeout(() => el.classList.remove("is-feeding"), 950);
  }

  // API pratique pour ton application :
  // window.langolierFeed(18)
  // Tu peux l'appeler quand le RAG termine réellement l'ingestion d'un fichier.
  window.langolierFeed = (amount = 11) => feedDocuments(amount);


  on(window, "pointermove", (e) => {
    updatePointer(e.clientX, e.clientY);
  }, { passive: true });

  on(window, "pointerleave", neutralPointer);

  on(el, "pointerenter", () => {
    targetHover = 1;
  });

  on(el, "pointerleave", () => {
    targetHover = 0;
  });

  on(el, "keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      feedDocuments(11);
    }
  });

  // Quand l'onglet n'est plus visible, on neutralise doucement le regard.
  on(document, "visibilitychange", () => {
    if (document.hidden) neutralPointer();
  });

  chooseNewIdleTarget(performance.now());
  requestAnimationFrame(frame);

  // What the host drives: a burst on demand, a steady stream while work is
  // happening, and a way to take it all back down.
  return {
    feed: function (amount) { feedDocuments(amount || 11); },
    // `on` here means "documents are being swallowed", not "the creature is
    // alive": it breathes either way.
    setFeeding: function (state) {
      if (state && !feedTimer) {
        feedDocuments(7);
        feedTimer = setInterval(function () {
          if (!document.hidden) feedDocuments(5 + Math.round(Math.random() * 4));
        }, 2600);
      } else if (!state && feedTimer) {
        clearInterval(feedTimer);
        feedTimer = null;
      }
    },
    destroy: function () {
      running = false;
      if (feedTimer) clearInterval(feedTimer);
      teardown.forEach(function (undo) { undo(); });
      teardown = [];
    },
  };
}
window.mountLangolier = mountLangolier;
