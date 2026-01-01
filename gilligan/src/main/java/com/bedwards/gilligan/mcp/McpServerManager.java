package com.bedwards.gilligan.mcp;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bedwards.gilligan.mcp.tool.*;
import com.bitwig.extension.controller.api.ControllerHost;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.modelcontextprotocol.server.McpServer;
import io.modelcontextprotocol.server.McpSyncServer;
import io.modelcontextprotocol.server.transport.HttpServletSseServerTransportProvider;
import io.modelcontextprotocol.spec.McpSchema;
import org.eclipse.jetty.server.Server;
import org.eclipse.jetty.server.ServerConnector;
import org.eclipse.jetty.servlet.ServletContextHandler;
import org.eclipse.jetty.servlet.ServletHolder;
import org.eclipse.jetty.util.thread.QueuedThreadPool;

/**
 * MCP Server Manager for Gilligan
 *
 * Hosts an MCP (Model Context Protocol) server that enables Claude Code
 * and other AI assistants to control Bitwig Studio via HTTP.
 *
 * Endpoint: http://localhost:61170/mcp (SSE at /sse)
 *
 * Token Optimization (following Anthropic 2025 best practices):
 * - 7 focused tools (vs WigAI's 15+) = ~70% less token overhead
 * - Concise descriptions (~10 words each)
 * - Minimal response payloads
 */
public class McpServerManager {

    private static final int DEFAULT_PORT = 61170;

    private final ControllerHost host;
    private final BitwigApiFacade facade;
    private Server jettyServer;
    private McpSyncServer mcpServer;
    private int port = DEFAULT_PORT;

    public McpServerManager(ControllerHost host, BitwigApiFacade facade) {
        this.host = host;
        this.facade = facade;
    }

    public void setPort(int port) {
        this.port = port;
    }

    public int getPort() {
        return port;
    }

    public void start() throws Exception {
        host.println("Gilligan MCP: Starting server on port " + port);

        try {
            // Create transport provider for SSE communication
            ObjectMapper objectMapper = new ObjectMapper();
            host.println("Gilligan MCP: Creating SSE transport (message=/mcp, sse=/sse)");
            HttpServletSseServerTransportProvider transportProvider =
                new HttpServletSseServerTransportProvider(objectMapper, "/mcp", "/sse");

            // Build MCP server
            host.println("Gilligan MCP: Building MCP server...");
            mcpServer = McpServer.sync(transportProvider)
                .serverInfo("Gilligan", "0.1.0")
                .capabilities(McpSchema.ServerCapabilities.builder()
                    .tools(true)
                    .resources(false, false)
                    .prompts(false)
                    .build())
                .build();

            // Register tools
            host.println("Gilligan MCP: Registering tools...");
            registerTools();
            host.println("Gilligan MCP: " + getToolCount() + " tools registered");

            // Create and configure Jetty server
            host.println("Gilligan MCP: Configuring Jetty server...");
            jettyServer = new Server(new QueuedThreadPool());
            ServerConnector connector = new ServerConnector(jettyServer);
            connector.setPort(port);
            jettyServer.addConnector(connector);

            // Set up servlet context
            ServletContextHandler context = new ServletContextHandler();
            context.setContextPath("/");
            context.addServlet(new ServletHolder(transportProvider), "/*");
            jettyServer.setHandler(context);

            // Start the server
            jettyServer.start();
            host.println("Gilligan MCP: Server running at http://localhost:" + port + "/mcp");
            host.println("Gilligan MCP: SSE endpoint at http://localhost:" + port + "/sse");
            host.println("Gilligan MCP: Claude Code config should use: {\"url\": \"http://localhost:" + port + "/sse\", \"transport\": \"sse\"}");
        } catch (Exception e) {
            host.errorln("Gilligan MCP: FAILED to start server: " + e.getClass().getName() + ": " + e.getMessage());
            for (StackTraceElement ste : e.getStackTrace()) {
                if (ste.getClassName().contains("gilligan") || ste.getClassName().contains("mcp")) {
                    host.errorln("  at " + ste);
                }
            }
            throw e;
        }
    }

    private int getToolCount() {
        // Count of registered tools
        return 9; // transport(4) + track(4) + device(1)
    }

    private void registerTools() {
        // Transport tools (minimal, focused)
        mcpServer.addTool(TransportPlayTool.create(facade, host));
        mcpServer.addTool(TransportStopTool.create(facade, host));
        mcpServer.addTool(TransportRecordTool.create(facade, host));
        mcpServer.addTool(GetTransportStateTool.create(facade, host));

        // Track tools
        mcpServer.addTool(ListTracksTool.create(facade, host));
        mcpServer.addTool(GetSelectedTrackTool.create(facade, host));
        mcpServer.addTool(CreateTrackTool.create(facade, host));
        mcpServer.addTool(RenameTrackTool.create(facade, host));

        // Device tools
        mcpServer.addTool(GetSelectedDeviceTool.create(facade, host));
    }

    public void stop() {
        host.println("Gilligan MCP: Stopping server...");
        try {
            if (jettyServer != null) {
                host.println("Gilligan MCP: Stopping Jetty server...");
                jettyServer.stop();
                jettyServer.join(); // Wait for full shutdown
                jettyServer = null;
                host.println("Gilligan MCP: Jetty server stopped");
            }
            if (mcpServer != null) {
                host.println("Gilligan MCP: Closing MCP server...");
                mcpServer.close();
                mcpServer = null;
                host.println("Gilligan MCP: MCP server closed");
            }
            host.println("Gilligan MCP: Server stopped successfully");
        } catch (Exception e) {
            host.errorln("Gilligan MCP: Error stopping server: " + e.getClass().getName() + ": " + e.getMessage());
            for (StackTraceElement ste : e.getStackTrace()) {
                if (ste.getClassName().contains("gilligan") || ste.getClassName().contains("jetty")) {
                    host.errorln("  at " + ste);
                }
            }
        }
    }

    public void restart() throws Exception {
        stop();
        start();
    }
}
