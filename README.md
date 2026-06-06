# 🎙️ **Gemini Live Assistant** - Asistente IA en Tiempo Real

Un asistente de inteligencia artificial conversacional en **tiempo real** que combina transcripción de voz, procesamiento de lenguaje natural y síntesis de voz usando **Google Gemini 2.5 Flash**, **FastAPI**, **Vue.js** y **WebSocket**.

## ✨ Características Principales

### 🎯 **Conversación Natural**
- Comprensión de audio bidireccional en múltiples idiomas
- Respuestas de IA en streaming (transmisión en vivo)
- Conversación continua sin necesidad de reactivación

### 🌍 **Multiidioma**
- Soporte automático para múltiples idiomas (inglés, español, etc.)
- Detección automática de idioma del usuario
- Respuestas en el mismo idioma

### 🔊 **Tecnología Avanzada**
- **STT (Speech-to-Text)**: Web Speech API del navegador
- **LLM**: Google Gemini 2.5 Flash con streaming
- **TTS (Text-to-Speech)**: Web Speech API + Google Cloud Text-to-Speech
- **WebSocket**: Comunicación en tiempo real servidor-cliente
- **RAG Simulado**: Base de contexto empresarial

### 🎨 **Interfaz Intuitiva**
- Visualizador de frecuencia de audio en vivo
- Diseño inmersivo estilo Gemini Live (tema oscuro)
- Estados visuales: escuchando, procesando, respondiendo
- Historial de conversación scrollable

---

## 📋 Requisitos Previos

- **Python 3.9+**
- **Node.js** (opcional, si usas frontend separado)
- **Google API Key** (para Gemini y Text-to-Speech)
- **Navegador moderno** con soporte WebSocket y Web Audio API

---

## 🚀 Instalación

### 1️⃣ Clonar el Repositorio

```bash
git clone https://github.com/Dan178A/tts_realtime.git
cd tts_realtime
```

### 2️⃣ Configurar Variables de Entorno

Crea un archivo `.env` en la raíz del proyecto:

```bash
GEMINI_API_KEY=tu_api_key_de_google_aqui
```

**Obtener API Key:**
- Ve a [Google Cloud Console](https://console.cloud.google.com/)
- Habilita la API de **Gemini** y **Text-to-Speech**
- Crea una clave API

### 3️⃣ Instalar Dependencias Python

```bash
pip install -r requirements.txt
```

**O manualmente:**

```bash
pip install fastapi uvicorn websockets python-dotenv google-genai google-cloud-texttospeech
```

### 4️⃣ Ejecutar el Servidor

```bash
python main.py
```

o con uvicorn directamente:

```bash
uvicorn main:app --reload --host 0.0.0.0 --port 8000
```

---

## 🎮 Uso

### **Opción 1: Interfaz Web Principal** (`index.html`)

Abre en tu navegador:
```
http://localhost:8000/index.html
```

1. Haz clic en **"INICIAR SISTEMA"**
2. Di **"asistente"** para activar
3. ¡Comienza a conversar! 🎤

**Comandos:**
- Di **"asistente"** → Activa el modo conversación
- Di **"adiós"** o **"apagar"** → Vuelve al modo durmiente

### **Opción 2: Interfaz Híbrida** (`test.html`)

Abre en tu navegador:
```
http://localhost:8000/test.html
```

Esta versión envía **audio binario real** a Gemini para transcripción más precisa.

---

## 📁 Estructura del Proyecto

```
tts_realtime/
├── main.py              # Backend FastAPI - Modo transcripción en Frontend
├── test.py              # Backend FastAPI - Modo audio binario a Gemini
├── chirp3.py            # Utilidad de síntesis de voz con Google Cloud
├── index.html           # Frontend principal (STT en navegador)
├── test.html            # Frontend hibrido (audio binario)
├── .env                 # Variables de entorno (no incluir en git)
├── .gitignore          # Archivos a ignorar
└── README.md           # Este archivo
```

---

## 🔄 Flujo de Funcionamiento

### **Arquitectura General**

```
┌─────────────────────┐
│   Navegador (Vue)   │
├─────────────────────┤
│ • Web Speech API    │
│ • WebSocket         │
│ • Canvas (Visual)   │
└──────────┬──────────┘
           │ WebSocket
           ▼
┌─────────────────────┐
│  FastAPI Server     │
├─────────────────────┤
│ • Gemini 2.5 Flash  │
│ • Streaming         │
│ • RAG (simulado)    │
└──────────┬──────────┘
           │
           ▼
    ┌──────────────┐
    │ Google Cloud │
    │  - Gemini    │
    │  - TTS       │
    └──────────────┘
```

### **Flujo de Conversación (main.py - Modo Rápido)**

1. **Usuario habla** → Web Speech API transcribe en frontend
2. **Frontend envía texto** → WebSocket al servidor
3. **Servidor ejecuta RAG** → Busca contexto relevante
4. **Gemini procesa** → Con contexto + prompt
5. **Streaming en vivo** → Servidor envía chunks de respuesta
6. **Frontend reproduce** → TTS del navegador lee la respuesta
7. **Loop continuo** → Vuelve a escuchar automáticamente

### **Flujo Híbrido (test.py - Modo Precisión)**

1. **Usuario habla** → MediaRecorder captura audio binario
2. **Frontend envía Blob** → WebSocket binario al servidor
3. **Gemini transcribe** → Audio real → Texto preciso
4. **Resto igual** → Procesa con RAG y genera respuesta
5. **Streaming + TTS** → Igual que arriba

---

## 🛠️ Configuración Avanzada

### Cambiar Palabra de Activación

**En `index.html` (línea 66):**
```javascript
palabraActivacion: 'tu_palabra',
```

### Cambiar Modelo de Gemini

**En `main.py` (línea 51):**
```python
model='gemini-2.0-flash-exp',  # o el modelo que prefieras
```

### Modificar Contexto RAG

**En `main.py` (línea 19):**
```python
return "Tu contexto empresarial personalizado aquí"
```

### Ajustar Velocidad de Voz

**En `index.html` (línea 235):**
```javascript
locucion.rate = 1.1;  // Rango: 0.5 - 2.0
```

---

## 📊 Ejemplos de Uso

### Ejemplo 1: Consulta Empresarial

```
Usuario: "¿Cuál es tu horario de atención?"
Asistente: "Nuestro horario es 24/7, estamos disponibles las 24 horas del día..."
```

### Ejemplo 2: Conversación Multiidioma

```
Usuario: "Good morning, what languages do you support?"
Asistente: "We support multiple languages including English and Spanish..."
```

### Ejemplo 3: Conversación Continua

```
Usuario: "¿Cómo estás?"
Asistente: "¡Estoy bien, gracias! ¿En qué puedo ayudarte?"
Usuario: "Cuéntame un chiste"
Asistente: (Automáticamente escuchando y respondiendo)
```

---

## 🐛 Troubleshooting

### ❌ Error: "Necesitas dar permisos de micrófono"

**Solución:** Permite el acceso al micrófono cuando el navegador lo solicite.

### ❌ Error: "GEMINI_API_KEY no encontrada"

**Solución:** Verifica que:
1. Creaste el archivo `.env`
2. Agregaste `GEMINI_API_KEY=tu_clave_aqui`
3. Ejecutas `dotenv.load_dotenv()` en el servidor


---

## 🎓 Conceptos Clave

### **WebSocket**
- Conexión bidireccional persistente
- Permite streaming en tiempo real
- Bajo overhead en comparación con HTTP polling

### **Streaming de Gemini**
- Respuestas fragmentadas en chunks
- Menor latencia percibida por el usuario
- Sensación de respuesta "en vivo"

### **RAG (Retrieval Augmented Generation)**
- Búsqueda de contexto + Generación de IA
- Mejora precisión de respuestas
- En este proyecto es simulado (mejora futura)

### **Web Speech API**
- STT: Speech Recognition (transcripción)
- TTS: Speech Synthesis (síntesis de voz)
- Sin dependencias externas en el navegador

---

## 📈 Mejoras Futuras

- [ ] Integración de base de datos real (PostgreSQL)
- [ ] Sistema RAG con embeddings (Pinecone/Weaviate)
- [ ] Persistencia de conversaciones
- [ ] Sistema de autenticación de usuarios
- [ ] Interfaz de administración
- [ ] Estadísticas y analytics
- [ ] Soporte para múltiples idiomas UI
- [ ] Caché de respuestas
- [ ] Rate limiting inteligente

---

## 📜 Licencia

Este proyecto está bajo licencia **MIT**. Ver `LICENSE` para más detalles.

---

## 👤 Autor

Creado por **Dan178A**

---

## 🤝 Contribuciones

Las contribuciones son bienvenidas. Por favor:

1. Haz un **Fork** del proyecto
2. Crea una rama (`git checkout -b feature/mejora`)
3. Commit tus cambios (`git commit -am 'Agrega mejora'`)
4. Push a la rama (`git push origin feature/mejora`)
5. Abre un **Pull Request**

---

## 💬 Soporte

Si tienes preguntas o problemas:
- Abre un **Issue** en el repositorio
- Revisa la sección de Troubleshooting
- Consulta la documentación de [Google Gemini](https://ai.google.dev/)

---

## ⭐ Si te fue útil, ¡dale una estrella! ⭐

```
🌟 tts_realtime - Asistente IA conversacional en tiempo real 🌟
```

---

**Última actualización:** Junio 2026
