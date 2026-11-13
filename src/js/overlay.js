export class OverlayCardController {
    constructor({ onReselect, onClose, onLanguageChange }) {
        this.card = document.getElementById('translation-card');
        this.dragHandle = document.getElementById('card-drag-handle');
        this.loadingState = document.getElementById('loading-state');
        this.resultState = document.getElementById('result-state');
        this.translatedOutput = document.getElementById('translated-output');
        this.originalOutput = document.getElementById('original-output');
        this.langSelect = document.getElementById('lang-select');
        this.reselectBtn = document.getElementById('reselect-btn');
        this.copyBtn = document.getElementById('copy-btn');
        this.closeBtn = document.getElementById('close-btn');

        this.currentOriginal = '';
        this.currentTranslated = '';

        this._initEvents(onReselect, onClose, onLanguageChange);
        this._initDrag();
    }

    _initEvents(onReselect, onClose, onLanguageChange) {
        this.reselectBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            if (onReselect) onReselect();
        });

        this.closeBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            if (onClose) onClose();
        });

        this.copyBtn.addEventListener('click', async (e) => {
            e.stopPropagation();
            if (!this.currentTranslated) return;
            try {
                if (window.__TAURI__?.core?.invoke) {
                    await window.__TAURI__.core.invoke('copy_clipboard', { text: this.currentTranslated });
                } else {
                    await navigator.clipboard.writeText(this.currentTranslated);
                }
                const prev = this.copyBtn.textContent;
                this.copyBtn.textContent = '✓ Скопировано';
                setTimeout(() => { this.copyBtn.textContent = prev; }, 1400);
            } catch (err) {
                console.error('Clipboard error:', err);
            }
        });

        this.langSelect.addEventListener('change', (e) => {
            e.stopPropagation();
            if (onLanguageChange && this.currentOriginal) {
                onLanguageChange(this.currentOriginal, this.langSelect.value);
            }
        });

        this.card.addEventListener('mousedown', (e) => {
            e.stopPropagation();
        });
    }

    _initDrag() {
        let dragging = false;
        let offsetX = 0;
        let offsetY = 0;

        this.dragHandle.addEventListener('mousedown', (e) => {
            if (e.target.tagName === 'BUTTON' || e.target.tagName === 'SELECT') return;
            dragging = true;
            const rect = this.card.getBoundingClientRect();
            offsetX = e.clientX - rect.left;
            offsetY = e.clientY - rect.top;
            e.stopPropagation();
        });

        window.addEventListener('mousemove', (e) => {
            if (!dragging) return;
            const x = Math.max(8, Math.min(window.innerWidth - this.card.offsetWidth - 8, e.clientX - offsetX));
            const y = Math.max(8, Math.min(window.innerHeight - this.card.offsetHeight - 8, e.clientY - offsetY));
            this.card.style.left = `${x}px`;
            this.card.style.top = `${y}px`;
        });

        window.addEventListener('mouseup', () => {
            dragging = false;
        });
    }

    positionNearRect(rect) {
        const cardW = 440;
        const cardH = 220;
        let left = Math.round(rect.left);
        let top = Math.round(rect.bottom + 12);

        if (left + cardW > window.innerWidth - 16) {
            left = Math.max(16, window.innerWidth - cardW - 16);
        }
        if (top + cardH > window.innerHeight - 16) {
            top = Math.max(16, Math.round(rect.top - cardH - 12));
        }

        this.card.style.left = `${left}px`;
        this.card.style.top = `${top}px`;
    }

    showLoading(rect) {
        if (rect) this.positionNearRect(rect);
        this.card.classList.remove('hidden');
        this.loadingState.classList.remove('hidden');
        this.resultState.classList.add('hidden');
    }

    showResult(original, translated) {
        this.currentOriginal = original || '';
        this.currentTranslated = translated || '';
        this.translatedOutput.textContent = this.currentTranslated;
        this.originalOutput.textContent = this.currentOriginal || '—';
        this.loadingState.classList.add('hidden');
        this.resultState.classList.remove('hidden');
        this.card.classList.remove('hidden');
    }

    showError(errMsg) {
        this.currentTranslated = '';
        this.translatedOutput.textContent = `⚠ ${errMsg}`;
        this.loadingState.classList.add('hidden');
        this.resultState.classList.remove('hidden');
        this.card.classList.remove('hidden');
    }

    hide() {
        this.card.classList.add('hidden');
    }

    getTargetLang() {
        return this.langSelect.value || 'auto';
    }
}
