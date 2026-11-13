import { OverlayCardController } from './overlay.js';

let startX = 0;
let startY = 0;
let isSelecting = false;
let selectionMode = true;

const overlayMask = document.getElementById('overlay-mask');
const hudHint = document.getElementById('hud-hint');
const selectionBox = document.getElementById('selection-box');
const selectionSize = document.getElementById('selection-size');

const invoke = (cmd, args) => {
    if (window.__TAURI__?.core?.invoke) {
        return window.__TAURI__.core.invoke(cmd, args);
    }
    return Promise.reject('Tauri API unavailable');
};

const closeApp = () => {
    invoke('exit_app').catch(() => {
        if (window.__TAURI__?.window?.getCurrentWindow) {
            window.__TAURI__.window.getCurrentWindow().close();
        }
    });
};

const enterSelectionMode = () => {
    selectionMode = true;
    cardController.hide();
    selectionBox.classList.add('hidden');
    overlayMask.classList.remove('transparent-mode');
    hudHint.classList.remove('hidden');
};

const cardController = new OverlayCardController({
    onReselect: () => enterSelectionMode(),
    onClose: () => closeApp(),
    onLanguageChange: async (originalText, newLang) => {
        cardController.showLoading(null);
        try {
            const retranslated = await invoke('retranslate_text', {
                text: originalText,
                targetLang: newLang,
            });
            cardController.showResult(originalText, retranslated);
        } catch (err) {
            cardController.showError(String(err));
        }
    },
});

window.addEventListener('mousedown', (e) => {
    if (!selectionMode || e.button !== 0) return;

    isSelecting = true;
    startX = e.clientX;
    startY = e.clientY;

    selectionBox.style.left = `${startX}px`;
    selectionBox.style.top = `${startY}px`;
    selectionBox.style.width = '0px';
    selectionBox.style.height = '0px';
    selectionSize.textContent = '0 × 0';
    selectionBox.classList.remove('hidden');
});

window.addEventListener('mousemove', (e) => {
    if (!isSelecting) return;

    const currentX = e.clientX;
    const currentY = e.clientY;

    const width = Math.abs(currentX - startX);
    const height = Math.abs(currentY - startY);
    const left = Math.min(currentX, startX);
    const top = Math.min(currentY, startY);

    selectionBox.style.left = `${left}px`;
    selectionBox.style.top = `${top}px`;
    selectionBox.style.width = `${width}px`;
    selectionBox.style.height = `${height}px`;
    selectionSize.textContent = `${Math.round(width)} × ${Math.round(height)}`;
});

window.addEventListener('mouseup', async (e) => {
    if (!isSelecting) return;
    isSelecting = false;

    const rect = selectionBox.getBoundingClientRect();
    selectionBox.classList.add('hidden');

    if (rect.width < 14 || rect.height < 14) {
        return;
    }

    selectionMode = false;
    hudHint.classList.add('hidden');
    overlayMask.classList.add('transparent-mode');

    const dpr = window.devicePixelRatio || 1;
    const captureX = Math.round((window.screenX || 0) + rect.left * dpr);
    const captureY = Math.round((window.screenY || 0) + rect.top * dpr);
    const captureW = Math.round(rect.width * dpr);
    const captureH = Math.round(rect.height * dpr);

    cardController.showLoading(rect);

    try {
        const res = await invoke('capture_and_translate', {
            x: captureX,
            y: captureY,
            width: captureW,
            height: captureH,
            targetLang: cardController.getTargetLang(),
        });
        cardController.showResult(res.original, res.translated);
    } catch (err) {
        cardController.showError(String(err));
    }
});

window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
        closeApp();
    }
});
