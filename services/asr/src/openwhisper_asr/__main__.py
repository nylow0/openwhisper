"""ASR Worker entry point."""
import sys
import json
import logging
from typing import Dict, Any

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    stream=sys.stderr
)
logger = logging.getLogger(__name__)


def handle_message(msg: Dict[str, Any]) -> Dict[str, Any]:
    """Handle incoming message."""
    msg_type = msg.get("type")
    
    if msg_type == "health.check":
        return {
            "type": "health.ok",
            "timestamp": msg.get("timestamp"),
            "status": "ready",
            "version": "0.1.0"
        }
    elif msg_type == "echo":
        return {
            "type": "echo.response",
            "text": msg.get("text", "")
        }
    else:
        return {
            "type": "error",
            "message": f"Unknown message type: {msg_type}"
        }


def main():
    """Main entry point."""
    logger.info("ASR Worker starting...")
    
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        
        try:
            msg = json.loads(line)
            logger.debug(f"Received: {msg}")
            
            response = handle_message(msg)
            print(json.dumps(response), flush=True)
            
        except json.JSONDecodeError as e:
            logger.error(f"Invalid JSON: {e}")
            print(json.dumps({"type": "error", "message": "Invalid JSON"}), flush=True)
        except Exception as e:
            logger.exception("Error handling message")
            print(json.dumps({"type": "error", "message": str(e)}), flush=True)
    
    logger.info("ASR Worker shutting down...")


if __name__ == "__main__":
    main()