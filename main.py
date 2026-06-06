import asyncio
import os
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from google import genai
import dotenv
# ¡Recuerda usar variables de entorno para tu API Key en producción!
## necesito leer el .env
dotenv.load_dotenv()
os.environ["GEMINI_API_KEY"] = os.getenv("GEMINI_API_KEY")

app = FastAPI()
client = genai.Client()

# ==========================================
# MÓDULO RAG (Simulado)
# ==========================================
async def buscar_en_rag(query: str) -> str:
    await asyncio.sleep(0.05) 
    return "Información de la empresa: Nuestro horario de atención es 24/7 y soportamos inglés y español."

# ==========================================
# WEBSOCKET ENDPOINT
# ==========================================
@app.websocket("/ws/chat")
async def websocket_endpoint(websocket: WebSocket):
    await websocket.accept()
    print("Nuevo cliente conectado (Modo Texto Rápido).")
    
    try:
        while True:
            # 1. Recibimos el texto ya transcrito desde el Frontend
            texto_usuario = await websocket.receive_text()
            print(f"🎙️ Usuario (Vía Front) dijo: {texto_usuario}")

            # 2. Ejecutar la búsqueda RAG
            contexto_rag = await buscar_en_rag(texto_usuario)

            # 3. Construir el Prompt
            prompt_final = f"""
            Eres un asistente de voz en tiempo real. Responde de forma natural y conversacional. 
            Responde en el mismo idioma en el que te habla el usuario (detecta el idioma por el texto).
            
            <contexto>
            {contexto_rag}
            </contexto>
            
            Usuario: {texto_usuario}
            """

            # 4. Llamar a Gemini en modo Streaming
            response_stream = await client.aio.models.generate_content_stream(
                model='gemini-2.5-flash',
                contents=prompt_final,
            )

            # 5. Enviar la respuesta en vivo
            async for chunk in response_stream:
                if chunk.text:
                    await websocket.send_text(chunk.text)

            await websocket.send_text("[FIN_RESPUESTA]")
            print("✅ Respuesta finalizada.")

    except WebSocketDisconnect:
        print("❌ Cliente desconectado.")
    except Exception as e:
        print(f"⚠️ Error: {e}")