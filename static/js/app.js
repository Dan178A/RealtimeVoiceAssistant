/* =========================================================
   Gemini Live & RAG Voice Assistant — Vue 2 Application
   ========================================================= */

new Vue({
    el: '#app',
    data: {
        // App settings
        sidebarOpen: false,
        mode: 'texto_stt', // 'texto_stt' | 'audio_hibrido'
        estado: 'apagado', // apagado | durmiendo | escuchando_live | grabando_comando | procesando | hablando
        palabraActivacion: 'asistente',

        // RAG management
        documents: [],
        newDocText: '',
        submittingDoc: false,

        // WS & Logic
        ws: null,
        reconocimiento: null,
        textoTemporal: '',
        mensajes: [],
        silenceTimer: null,
        comandoEnviado: false,

        // Audio & Visualizer
        audioContext: null,
        analyser: null,
        microphone: null,
        streamGlobal: null,

        // Hybrid mode recorder
        mediaRecorder: null,
        audioChunks: [],

        // Browser-side voice synthesis
        vozSeleccionada: null,
        fraseAcumulada: '',
    },

    computed: {
        stateClass() {
            return `state-${this.estado}`;
        },
        statusText() {
            if (this.estado === 'apagado')          return 'Asistente Desconectado';
            if (this.estado === 'durmiendo')         return `Modo Pasivo: di "${this.palabraActivacion}"`;
            if (this.estado === 'escuchando_live')   return 'Modo Activo: Escuchando...';
            if (this.estado === 'grabando_comando')  return 'Grabando audio real...';
            if (this.estado === 'procesando')        return 'Procesando en el Servidor';
            if (this.estado === 'hablando')          return 'Reproduciendo Respuesta';
            return 'Listo';
        },
    },

    mounted() {
        this.configurarVoz();
        this.fetchDocuments();
    },

    methods: {
        /* ── Theme helpers ─────────────────────────────── */
        getThemeColor() {
            return this.mode === 'audio_hibrido' ? '#00bfff' : '#8a2be2';
        },
        getThemeGlow() {
            return this.mode === 'audio_hibrido'
                ? 'rgba(0, 191, 255, 0.4)'
                : 'rgba(138, 43, 226, 0.4)';
        },
        setMode(m) {
            this.mode = m;
        },

        /* ── Voice synthesis ───────────────────────────── */
        configurarVoz() {
            if (!window.speechSynthesis) return;
            const selectVoice = () => {
                const voces = window.speechSynthesis.getVoices();
                this.vozSeleccionada =
                    voces.find(v => v.name.includes('Google') && v.lang.startsWith('es')) ||
                    voces.find(v => v.lang.startsWith('es')) ||
                    null;
            };
            selectVoice();
            window.speechSynthesis.onvoiceschanged = selectVoice;
        },

        reproducirVoz(texto) {
            if (!window.speechSynthesis) return;
            const textoLimpio = texto.replace(/<[^>]*>?/gm, '').replace(/\*/g, '');
            if (!textoLimpio.trim()) return;
            const locucion = new SpeechSynthesisUtterance(textoLimpio);
            if (this.vozSeleccionada) locucion.voice = this.vozSeleccionada;
            locucion.rate = 1.1;
            window.speechSynthesis.speak(locucion);
        },

        playActivationSound() {
            if (!this.audioContext) return;
            const osc  = this.audioContext.createOscillator();
            const gain = this.audioContext.createGain();
            osc.connect(gain);
            gain.connect(this.audioContext.destination);
            osc.frequency.setValueAtTime(600, this.audioContext.currentTime);
            osc.frequency.exponentialRampToValueAtTime(900, this.audioContext.currentTime + 0.15);
            gain.gain.setValueAtTime(0.1, this.audioContext.currentTime);
            gain.gain.exponentialRampToValueAtTime(0.01, this.audioContext.currentTime + 0.15);
            osc.start();
            osc.stop(this.audioContext.currentTime + 0.15);
        },

        /* ── RAG API ───────────────────────────────────── */
        async fetchDocuments() {
            try {
                const res = await fetch('/api/documents');
                if (res.ok) this.documents = await res.json();
            } catch (err) {
                console.error('Error cargando documentos de RAG:', err);
            }
        },

        async addDocument() {
            if (!this.newDocText.trim()) return;
            this.submittingDoc = true;
            try {
                const res = await fetch('/api/documents', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ content: this.newDocText }),
                });
                if (res.ok) {
                    this.newDocText = '';
                    await this.fetchDocuments();
                } else {
                    alert('Error al guardar el documento.');
                }
            } catch (err) {
                console.error(err);
                alert('Error al conectar con la API de RAG.');
            } finally {
                this.submittingDoc = false;
            }
        },

        async deleteDocument(id) {
            if (!confirm('¿Deseas eliminar este documento del RAG?')) return;
            try {
                const res = await fetch(`/api/documents/${id}`, { method: 'DELETE' });
                if (res.ok) await this.fetchDocuments();
            } catch (err) {
                console.error(err);
            }
        },

        /* ── Chat helpers ──────────────────────────────── */
        clearHistory() {
            this.mensajes = [];
        },

        scrollAbajo() {
            this.$nextTick(() => {
                const container = this.$refs.historyContainer;
                if (container) container.scrollTop = container.scrollHeight;
            });
        },

        /* ── WebSocket & assistant core ────────────────── */
        async iniciarSistema() {
            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const host = window.location.host || 'localhost:8000';
            this.ws = new WebSocket(`${protocol}//${host}/ws/chat`);

            this.ws.onopen = () => {
                this.estado = 'durmiendo';
                this.setupMicrophone();
            };

            this.ws.onmessage = (event) => {
                const data = event.data;

                if (data.startsWith('[TRANSCRIPCION]')) {
                    const text = data.replace('[TRANSCRIPCION] ', '');
                    this.mensajes.push({ tipo: 'user',      texto: text });
                    this.mensajes.push({ tipo: 'assistant', texto: '' });
                    this.scrollAbajo();
                } else if (data === '[FIN_RESPUESTA]') {
                    if (this.fraseAcumulada.trim()) this.reproducirVoz(this.fraseAcumulada);
                    this.fraseAcumulada = '';
                    setTimeout(() => {
                        if (this.mode === 'audio_hibrido') {
                            this.iniciarGrabacionAudio();
                        } else {
                            this.estado = 'escuchando_live';
                        }
                    }, 500);
                } else {
                    this.estado = 'hablando';
                    const ultimoMsg = this.mensajes[this.mensajes.length - 1];
                    if (ultimoMsg && ultimoMsg.tipo === 'assistant') {
                        ultimoMsg.texto += data.replace(/\n/g, '<br>');
                    }
                    this.fraseAcumulada += data;
                    if (data.includes('.') || data.includes(',') || data.includes('?') ||
                        data.includes('!') || data.includes(':')) {
                        this.reproducirVoz(this.fraseAcumulada);
                        this.fraseAcumulada = '';
                    }
                    this.scrollAbajo();
                }
            };

            this.ws.onclose = () => {
                this.estado = 'apagado';
                this.textoTemporal = '';
                if (this.reconocimiento) this.reconocimiento.stop();
                if (this.mediaRecorder && this.mediaRecorder.state !== 'inactive') {
                    this.mediaRecorder.stop();
                }
            };

            this.ws.onerror = (e) => {
                console.error('WebSocket Error:', e);
                this.estado = 'apagado';
            };
        },

        /* ── Microphone & Speech Recognition ──────────── */
        async setupMicrophone() {
            try {
                const stream = await navigator.mediaDevices.getUserMedia({
                    audio: { noiseSuppression: true, echoCancellation: true, autoGainControl: true },
                });
                this.streamGlobal = stream;
                this.iniciarVisualizador(stream);

                const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
                this.reconocimiento = new SpeechRecognition();
                this.reconocimiento.lang = 'es-ES';
                this.reconocimiento.continuous = true;
                this.reconocimiento.interimResults = true;

                this.reconocimiento.onresult = (event) => {
                    if (this.estado === 'hablando' || this.estado === 'procesando') return;

                    let transcripcionActual = '';
                    for (let i = event.resultIndex; i < event.results.length; i++) {
                        transcripcionActual += event.results[i][0].transcript;
                    }
                    const textoLimpio = transcripcionActual.trim().toLowerCase();

                    // ---------- MODO TEXTO (STT) ----------
                    if (this.mode === 'texto_stt') {
                        if (this.estado === 'durmiendo') {
                            if (textoLimpio.includes(this.palabraActivacion)) {
                                this.estado = 'escuchando_live';
                                this.textoTemporal = '';
                                this.playActivationSound();
                            }
                            return;
                        }
                        if (this.estado === 'escuchando_live') {
                            if (textoLimpio.includes('adiós') || textoLimpio.includes('apagar') ||
                                textoLimpio.includes('dormir')) {
                                this.estado = 'durmiendo';
                                this.textoTemporal = '';
                                return;
                            }
                            this.textoTemporal = transcripcionActual;
                            clearTimeout(this.silenceTimer);
                            this.silenceTimer = setTimeout(() => this.enviarComandoTexto(), 1500);
                        }
                    }
                    // ---------- MODO AUDIO HÍBRIDO ----------
                    else if (this.mode === 'audio_hibrido') {
                        if (this.estado === 'durmiendo') {
                            if (textoLimpio.includes(this.palabraActivacion)) {
                                this.iniciarGrabacionAudio();
                                this.playActivationSound();
                            }
                            return;
                        }
                        if (this.estado === 'grabando_comando' && !this.comandoEnviado) {
                            this.textoTemporal = transcripcionActual;
                            clearTimeout(this.silenceTimer);
                            this.silenceTimer = setTimeout(() => this.detenerYEnviarAudio(), 1500);
                        }
                    }
                };

                this.reconocimiento.onend = () => {
                    if (this.estado !== 'apagado') {
                        try { this.reconocimiento.start(); } catch (_) { /* already running */ }
                    }
                };

                this.reconocimiento.start();
            } catch (err) {
                console.error('Mic permissions error:', err);
                alert('Necesitas dar permisos de micrófono en tu navegador.');
                this.estado = 'apagado';
            }
        },

        /* ── Text command ──────────────────────────────── */
        enviarComandoTexto() {
            if (!this.textoTemporal.trim()) return;
            const txt = this.textoTemporal;
            this.mensajes.push({ tipo: 'user',      texto: txt });
            this.mensajes.push({ tipo: 'assistant', texto: '' });
            this.ws.send(txt);
            this.textoTemporal = '';
            this.estado = 'procesando';
            this.scrollAbajo();
        },

        /* ── Hybrid audio recording ────────────────────── */
        iniciarGrabacionAudio() {
            this.estado = 'grabando_comando';
            this.comandoEnviado = false;
            this.textoTemporal = '';
            this.audioChunks = [];

            this.mediaRecorder = new MediaRecorder(this.streamGlobal, { mimeType: 'audio/webm' });

            this.mediaRecorder.ondataavailable = (event) => {
                if (event.data.size > 0) this.audioChunks.push(event.data);
            };

            this.mediaRecorder.onstop = () => {
                if (this.audioChunks.length > 0) {
                    const audioBlob = new Blob(this.audioChunks, { type: 'audio/webm' });
                    this.ws.send(audioBlob);
                    this.estado = 'procesando';
                }
            };

            this.mediaRecorder.start();
        },

        detenerYEnviarAudio() {
            if (this.comandoEnviado) return;
            this.comandoEnviado = true;
            if (this.mediaRecorder && this.mediaRecorder.state !== 'inactive') {
                this.mediaRecorder.stop();
            }
        },

        /* ── Canvas Visualizer ─────────────────────────── */
        iniciarVisualizador(stream) {
            const canvas = this.$refs.canvas;
            if (!canvas) return;
            const ctx = canvas.getContext('2d');

            const dpr  = window.devicePixelRatio || 1;
            const rect = canvas.getBoundingClientRect();
            canvas.width  = rect.width  * dpr;
            canvas.height = rect.height * dpr;
            ctx.scale(dpr, dpr);

            this.audioContext = new (window.AudioContext || window.webkitAudioContext)();
            this.analyser     = this.audioContext.createAnalyser();
            this.microphone   = this.audioContext.createMediaStreamSource(stream);
            this.microphone.connect(this.analyser);
            this.analyser.fftSize = 512;

            const bufferLength = this.analyser.frequencyBinCount;
            const dataArray    = new Uint8Array(bufferLength);

            const draw = () => {
                requestAnimationFrame(draw);

                if (this.estado === 'apagado') {
                    ctx.clearRect(0, 0, rect.width, rect.height);
                    return;
                }

                this.analyser.getByteFrequencyData(dataArray);
                ctx.clearRect(0, 0, rect.width, rect.height);

                const cx = rect.width  / 2;
                const cy = rect.height / 2;
                const baseR = 65;

                let scale = 1;
                if (this.estado === 'escuchando_live' || this.estado === 'grabando_comando') scale = 1.3;
                else if (this.estado === 'hablando')   scale = 1.15;
                else if (this.estado === 'procesando') scale = 1.05;

                const grad = ctx.createLinearGradient(0, 0, rect.width, rect.height);
                if      (this.estado === 'escuchando_live')  { grad.addColorStop(0, '#ffd700'); grad.addColorStop(1, '#ffa500'); }
                else if (this.estado === 'grabando_comando') { grad.addColorStop(0, '#ff4a4a'); grad.addColorStop(1, '#b30000'); }
                else if (this.estado === 'hablando')         { grad.addColorStop(0, '#00ff7f'); grad.addColorStop(1, '#00bfff'); }
                else if (this.estado === 'procesando')       { grad.addColorStop(0, '#00bfff'); grad.addColorStop(1, '#8a2be2'); }
                else                                         { grad.addColorStop(0, '#5f6368'); grad.addColorStop(1, '#3c4043'); }

                // Outer wave ring
                ctx.beginPath();
                ctx.strokeStyle = grad;
                ctx.lineWidth   = 3;
                ctx.shadowBlur  = 15;
                ctx.shadowColor = this.getThemeColor();

                for (let i = 0; i < bufferLength; i++) {
                    const angle  = (i / bufferLength) * Math.PI * 2;
                    const value  = dataArray[i] / 255.0;
                    const r      = baseR + value * 45 * scale;
                    const x      = cx + Math.cos(angle) * r;
                    const y      = cy + Math.sin(angle) * r;
                    if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
                }
                ctx.closePath();
                ctx.stroke();

                // Inner core circle
                ctx.beginPath();
                ctx.fillStyle  = grad;
                ctx.shadowBlur = 0;
                ctx.arc(cx, cy, baseR - 5, 0, Math.PI * 2);
                ctx.fill();
            };

            draw();
        },
    },
});
