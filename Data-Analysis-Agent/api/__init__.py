"""Flask application factory."""
import os
from pathlib import Path

from flask import Flask, render_template, request, jsonify
from flask_cors import CORS


def create_app() -> Flask:
    root = Path(__file__).parent.parent
    app = Flask(
        __name__,
        template_folder=str(root / "templates"),
        static_folder=str(root / "static"),
    )
    app.secret_key = os.urandom(32)
    CORS(app)

    agent_token = (os.environ.get("AGENT_TOKEN") or "").strip()

    @app.before_request
    def _guard_sidecar_api():
        # Protect sidecar API calls with one-time token. Non-API pages (e.g. /dashboard) stay local-only.
        if not request.path.startswith("/api/"):
            return None

        if request.method == "OPTIONS":
            return None

        if not agent_token:
            return None

        auth = (request.headers.get("Authorization") or "").strip()
        if auth.lower().startswith("bearer "):
            incoming = auth[7:].strip()
        else:
            incoming = (request.headers.get("X-Agent-Token") or "").strip()

        if incoming != agent_token:
            return jsonify({"error": "Unauthorized"}), 401

        return None

    from .models          import bp as models_bp
    from .datasource      import bp as datasource_bp
    from .chat            import bp as chat_bp
    from .saved_sessions  import bp as saved_sessions_bp
    from .system          import bp as system_bp
    from .output          import bp as output_bp
    from .mcp             import bp as mcp_bp
    from .dashboard       import bp as dashboard_bp

    app.register_blueprint(models_bp)
    app.register_blueprint(datasource_bp)
    app.register_blueprint(chat_bp)
    app.register_blueprint(saved_sessions_bp)
    app.register_blueprint(system_bp)
    app.register_blueprint(output_bp)
    app.register_blueprint(mcp_bp)
    app.register_blueprint(dashboard_bp)

    @app.get("/")
    def index():
        return render_template("agent_chat.html")

    return app
