package com.bedwards.gilligan.rest;

import com.bedwards.gilligan.GilliganService;
import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.servlet.http.HttpServlet;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;

import java.io.IOException;
import java.util.Map;

/**
 * Simple REST API servlet for Gilligan.
 *
 * Endpoints:
 *   POST /api/{command}  - Execute command with JSON body as args
 *   GET  /api/{command}  - Execute command (no args)
 *   GET  /api            - List available commands
 *
 * No sessions, no SSE, just JSON in/out.
 */
public class RestApiServlet extends HttpServlet {

    private static final ObjectMapper mapper = new ObjectMapper();
    private final GilliganService service;

    public RestApiServlet(GilliganService service) {
        this.service = service;
    }

    @Override
    protected void doGet(HttpServletRequest req, HttpServletResponse resp) throws IOException {
        String path = req.getPathInfo();
        resp.setContentType("application/json");
        resp.setCharacterEncoding("UTF-8");

        if (path == null || path.equals("/") || path.isEmpty()) {
            // List available commands
            String help = """
                {
                  "commands": {
                    "play": "Start playback",
                    "stop": "Stop playback",
                    "record": "Toggle recording",
                    "transport": "Get transport state (tempo, position, playing)",
                    "tracks": "List all tracks",
                    "track": "Get selected track",
                    "device": "Get selected device",
                    "snapshot": "Get full project snapshot",
                    "create_track": "Create track (args: type=instrument|audio)",
                    "rename_track": "Rename selected track (args: name=...)",
                    "stage": "Stage programs (args: stages=[...], commitAt=next_bar)"
                  },
                  "usage": "GET/POST /api/{command}"
                }
                """;
            resp.getWriter().write(help);
            return;
        }

        String command = path.substring(1); // Remove leading /
        GilliganService.Result result = service.dispatch(command, null);

        resp.setStatus(result.success() ? 200 : 400);
        resp.getWriter().write(result.toJson());
    }

    @Override
    @SuppressWarnings("unchecked")
    protected void doPost(HttpServletRequest req, HttpServletResponse resp) throws IOException {
        String path = req.getPathInfo();
        resp.setContentType("application/json");
        resp.setCharacterEncoding("UTF-8");

        if (path == null || path.equals("/") || path.isEmpty()) {
            resp.setStatus(400);
            resp.getWriter().write("{\"error\": \"Command required: POST /api/{command}\"}");
            return;
        }

        String command = path.substring(1);

        // Parse JSON body as args
        Map<String, Object> args = null;
        String body = new String(req.getInputStream().readAllBytes());
        if (body != null && !body.trim().isEmpty()) {
            try {
                args = mapper.readValue(body, Map.class);
            } catch (Exception e) {
                resp.setStatus(400);
                resp.getWriter().write("{\"error\": \"Invalid JSON: " + e.getMessage() + "\"}");
                return;
            }
        }

        GilliganService.Result result = service.dispatch(command, args);

        resp.setStatus(result.success() ? 200 : 400);
        resp.getWriter().write(result.toJson());
    }

    @Override
    protected void doOptions(HttpServletRequest req, HttpServletResponse resp) {
        // CORS preflight
        resp.setHeader("Access-Control-Allow-Origin", "*");
        resp.setHeader("Access-Control-Allow-Methods", "GET, POST, OPTIONS");
        resp.setHeader("Access-Control-Allow-Headers", "Content-Type");
        resp.setStatus(200);
    }
}
