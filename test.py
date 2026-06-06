import asyncio
import os
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from google import genai
from google.genai import types
import dotenv

# Leer el .env
dotenv.load_dotenv()
os.environ["GEMINI_API_KEY"] = os.getenv("GEMINI_API_KEY")

app = FastAPI()
client = genai.Client()

# ==========================================
# MÓDULO RAG (Simulado)
# ==========================================
async def buscar_en_rag(query: str) -> str:
    await asyncio.sleep(0.05) 
    return "Información de la empresa: Nuestro horario de atención es 24/7 y soportamos múltiples idiomas."

# ==========================================
# WEBSOCKET ENDPOINT
# ==========================================
@app.websocket("/ws/chat")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    print("✅ Cliente conectado (Modo Audio Híbrido).")
    
    try:
        while True:
            # 1. Recibimos el AUDIO BINARIO desde Vue
            data = await websocket.receive()
            
            if "bytes" in data:
                audio_bytes = data["bytes"]
                print("🎙️ Audio recibido. Transcribiendo con Gemini...")

                # 2. Transcripción Multi-idioma con Gemini
                transcription_response = await client.aio.models.generate_content(
                    model='gemini-2.5-flash',
                    contents=[
                        types.Part.from_bytes(data=audio_bytes, mime_type='audio/webm'),
                        "Transcribe exactamente lo que se dice. Detecta el idioma automáticamente. Devuelve SOLO el texto de la transcripción, sin comillas ni notas."
                    ]
                )
                
                texto_usuario = transcription_response.text.strip()
                print(f"📝 Transcripción: {texto_usuario}")
                
                if not texto_usuario:
                    await websocket.send_text("[FIN_RESPUESTA]")
                    continue

                # 3. Enviamos la transcripción exacta a la interfaz web
                await websocket.send_text(f"[TRANSCRIPCION] {texto_usuario}")

                # 4. RAG y Generación de Respuesta
                contexto_rag = await buscar_en_rag(texto_usuario)

                prompt_final = f"""
                Eres un asistente de voz en tiempo real. Responde de forma natural y conversacional. 
                Responde en el mismo idioma en el que te habla el usuario.
                
                <contexto>
                {contexto_rag}
                </contexto>
                
                Usuario: {texto_usuario}
                """

                # 5. Streaming de la respuesta
                response_stream = await client.aio.models.generate_content_stream(
                    model='gemini-2.5-flash',
                    contents=prompt_final,
                )

                async for chunk in response_stream:
                    if chunk.text:
                        await websocket.send_text(chunk.text)

                await websocket.send_text("[FIN_RESPUESTA]")
                print("✅ Respuesta finalizada.\n")

    except WebSocketDisconnect:
        print("❌ Cliente desconectado.")
    except Exception as e:
        print(f"⚠️ Error: {e}")