(() => {
  "use strict";

  const header = document.querySelector(".site-header");
  const nav = document.getElementById("primary-nav");
  const navToggle = document.querySelector(".nav-toggle");
  const copyStatus = document.getElementById("copy-status");

  if (header) {
    const updateHeader = () => {
      header.classList.toggle("is-scrolled", window.scrollY > 8);
    };
    updateHeader();
    window.addEventListener("scroll", updateHeader, { passive: true });
  }

  if (nav && navToggle) {
    const setMenu = (open) => {
      nav.classList.toggle("is-open", open);
      navToggle.setAttribute("aria-expanded", String(open));
      navToggle.setAttribute("aria-label", open ? "Close menu" : "Open menu");
    };

    navToggle.addEventListener("click", () => {
      setMenu(!nav.classList.contains("is-open"));
    });

    nav.addEventListener("click", (event) => {
      if (event.target.closest("a")) setMenu(false);
    });

    document.addEventListener("click", (event) => {
      if (
        nav.classList.contains("is-open") &&
        !nav.contains(event.target) &&
        !navToggle.contains(event.target)
      ) {
        setMenu(false);
      }
    });

    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && nav.classList.contains("is-open")) {
        setMenu(false);
        navToggle.focus();
      }
    });

    const wide = window.matchMedia("(min-width: 1121px)");
    const onWide = (event) => {
      if (event.matches) setMenu(false);
    };
    if (wide.addEventListener) wide.addEventListener("change", onWide);
    else wide.addListener(onWide);
  }

  const activateTab = (tab, tabs) => {
    tabs.forEach((item) => {
      const selected = item === tab;
      item.setAttribute("aria-selected", String(selected));
      item.tabIndex = selected ? 0 : -1;
    });
  };

  const bindKeyboard = (tablist, tabs, select, vertical) => {
    tablist.addEventListener("keydown", (event) => {
      const current = tabs.indexOf(document.activeElement);
      if (current === -1) return;

      const nextKey = vertical ? "ArrowDown" : "ArrowRight";
      const prevKey = vertical ? "ArrowUp" : "ArrowLeft";
      let next = null;

      if (event.key === nextKey) next = (current + 1) % tabs.length;
      if (event.key === prevKey) next = (current - 1 + tabs.length) % tabs.length;
      if (event.key === "Home") next = 0;
      if (event.key === "End") next = tabs.length - 1;

      if (next !== null) {
        event.preventDefault();
        select(tabs[next]);
        tabs[next].focus();
      }
    });
  };

  const galleryTabs = Array.from(document.querySelectorAll(".shot-tab"));
  const galleryImage = document.getElementById("gallery-image");
  const galleryWindowTitle = document.getElementById("gallery-window-title");
  const galleryCaption = document.getElementById("gallery-caption");
  const galleryOpen = document.getElementById("gallery-open");

  if (galleryTabs.length && galleryImage && galleryCaption && galleryWindowTitle && galleryOpen) {
    const galleryPanel = document.getElementById("shot-panel") ||
      document.querySelector('[role="tabpanel"][aria-labelledby="tab-devices"]');
    const galleryTablist = document.querySelector('.shot-tabs[role="tablist"]');

    const selectShot = (tab) => {
      activateTab(tab, galleryTabs);

      galleryImage.src = tab.dataset.src;
      galleryImage.alt = tab.dataset.alt || "";
      galleryWindowTitle.textContent = tab.dataset.shot || "";
      galleryOpen.href = tab.dataset.full || tab.dataset.src;

      const numeric = tab.querySelector(".num");
      const number = numeric ? numeric.textContent.trim() : "";
      galleryCaption.innerHTML = "";

      const figure = document.createElement("strong");
      figure.textContent = `FIG. ${number} — ${(tab.dataset.shot || "").toUpperCase()}.`;
      galleryCaption.append(figure, ` ${tab.dataset.caption || ""}`);

      if (galleryPanel) galleryPanel.setAttribute("aria-labelledby", tab.id);
    };

    galleryTabs.forEach((tab) => {
      tab.addEventListener("click", () => selectShot(tab));
    });

    if (galleryTablist) {
      bindKeyboard(galleryTablist, galleryTabs, selectShot, false);
    }

    const preload = () => {
      galleryTabs.forEach((tab) => {
        if (!tab.dataset.src) return;
        const image = new Image();
        image.src = tab.dataset.src;
      });
    };

    if ("requestIdleCallback" in window) {
      window.requestIdleCallback(preload, { timeout: 2500 });
    } else {
      window.setTimeout(preload, 1200);
    }
  }

  const installTabs = Array.from(document.querySelectorAll(".install-tab"));

  if (installTabs.length) {
    const installTablist = document.querySelector('.install-tabs[role="tablist"]');

    const selectInstall = (tab) => {
      activateTab(tab, installTabs);

      installTabs.forEach((item) => {
        const panel = document.getElementById(item.getAttribute("aria-controls"));
        if (panel) panel.hidden = item !== tab;
      });
    };

    installTabs.forEach((tab) => {
      tab.addEventListener("click", () => selectInstall(tab));
    });

    if (installTablist) {
      bindKeyboard(installTablist, installTabs, selectInstall, false);
    }
  }

  const legacyCopy = (text) => {
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.top = "-1000px";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();

    let copied = false;
    try {
      copied = document.execCommand("copy");
    } catch (error) {
      copied = false;
    }

    textarea.remove();
    return copied;
  };

  const copyText = async (text) => {
    if (navigator.clipboard && window.isSecureContext) {
      try {
        await navigator.clipboard.writeText(text);
        return true;
      } catch (error) {
        // Fall through to the legacy path for browsers that deny clipboard access.
      }
    }
    return legacyCopy(text);
  };

  const copyButtons = Array.from(document.querySelectorAll("[data-copy]"));

  copyButtons.forEach((button) => {
    button.addEventListener("click", async () => {
      const target = document.getElementById(button.dataset.copy);
      if (!target) return;

      const text = target.innerText.trim();
      const copied = await copyText(text);
      if (!copied) return;

      const label = button.querySelector("span");
      const iconUse = button.querySelector("use");
      const previousLabel = label ? label.textContent : "";
      const previousIcon = iconUse ? iconUse.getAttribute("href") : "";

      button.classList.add("is-copied");
      if (label) label.textContent = "Copied";
      if (iconUse) iconUse.setAttribute("href", "#i-check");
      if (copyStatus) copyStatus.textContent = "Copied to clipboard";

      window.clearTimeout(button.copyTimer);
      button.copyTimer = window.setTimeout(() => {
        button.classList.remove("is-copied");
        if (label) label.textContent = previousLabel;
        if (iconUse && previousIcon) iconUse.setAttribute("href", previousIcon);
        if (copyStatus) copyStatus.textContent = "";
      }, 1600);
    });
  });
})();
