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

        // Create transport provider for SSE communication
        ObjectMapper objectMapper = new ObjectMapper();
        HttpServletSseServerTransportProvider transportProvider =
            new HttpServletSseServerTransportProvider(objectMapper, "/mcp", "/sse");

        // Build MCP server
        mcpServer = McpServer.sync(transportProvider)
            .serverInfo("Gilligan", "0.1.0")
            .capabilities(McpSchema.ServerCapabilities.builder()
                .tools(true)
                .resources(false, false)
                .prompts(false)
                .build())
            .build();

        // Register tools
        registerTools();

        // Create and configure Jetty server
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
        try {
            if (mcpServer != null) {
                mcpServer.close();
                mcpServer = null;
            }
            if (jettyServer != null) {
                jettyServer.stop();
                jettyServer = null;
            }
            host.println("Gilligan MCP: Server stopped");
        } catch (Exception e) {
            host.errorln("Gilligan MCP: Error stopping server: " + e.getMessage());
        }
    }

    public void restart() throws Exception {
        stop();
        start();
    }
}
