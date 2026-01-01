package com.bedwards.gilligan;

import com.bedwards.gilligan.mcp.McpServerManager;
import com.bitwig.extension.controller.ControllerExtension;
import com.bitwig.extension.controller.api.ControllerHost;

/**
 * Gilligan Extension
 *
 * Bitwig Controller Extension with MCP Server for AI-assisted music production.
 * Enables Claude Code and other AI assistants to control Bitwig Studio.
 *
 * ## MCP Server
 *
 * Hosts an MCP (Model Context Protocol) server at http://localhost:61170/mcp
 *
 * Tools (optimized for minimal token usage):
 * - transport_play: Start playback
 * - transport_stop: Stop playback
 * - transport_record: Toggle recording
 * - get_transport: Get tempo, time sig, position, status
 * - list_tracks: List tracks (name, color, position, type)
 * - get_selected_track: Get currently selected track
 * - get_selected_device: Get currently selected device
 *
 * ## Token Optimization
 *
 * Following Anthropic's MCP best practices (2025):
 * - Minimal tool set (7 tools vs WigAI's 15+)
 * - Concise descriptions (~10 words each)
 * - Filtered responses (only essential data)
 * - Progressive disclosure pattern ready
 *
 * ## Feature Comparison with Skipper
 *
 * | Feature                  | Skipper (CLAP) | Gilligan (MCP Server) |
 * |--------------------------|----------------|----------------------|
 * | Host name/version        | Yes            | Yes                  |
 * | Track info               | Limited*       | Full                 |
 * | Transport control        | Read-only      | Read/Write           |
 * | Device info              | No             | Yes                  |
 * | AI integration           | No             | MCP Server           |
 *
 * * = Requires CLAP track-info extension support
 */
public class GilliganExtension extends ControllerExtension {

    private BitwigApiFacade facade;
    private McpServerManager mcpServer;

    protected GilliganExtension(
            GilliganExtensionDefinition definition,
            ControllerHost host) {
        super(definition, host);
    }

    @Override
    public void init() {
        ControllerHost host = getHost();

        host.println("Gilligan: Initializing...");
        host.println("Gilligan: Bitwig API version " + host.getHostApiVersion());
        host.println("Gilligan: Bitwig version " + host.getHostVersion());

        // Create Bitwig API facade (sets up all observers)
        facade = new BitwigApiFacade(host);
        host.println("Gilligan: Bitwig API facade initialized");

        // Create and start MCP server
        mcpServer = new McpServerManager(host, facade);
        try {
            mcpServer.start();
            host.println("Gilligan: MCP server started at http://localhost:61170/mcp");
        } catch (Exception e) {
            host.errorln("Gilligan: Failed to start MCP server: " + e.getMessage());
        }

        host.println("Gilligan: Initialization complete");
        host.println("Gilligan: Ready for Claude Code integration");
    }

    @Override
    public void exit() {
        ControllerHost host = getHost();
        host.println("Gilligan: Shutting down...");

        // Stop MCP server
        if (mcpServer != null) {
            mcpServer.stop();
        }

        host.println("Gilligan: Shutdown complete");
    }

    @Override
    public void flush() {
        // Called when Bitwig wants us to update hardware state
        // For Gilligan, we primarily receive info and serve MCP requests
    }

    /**
     * Get the Bitwig API facade for direct access.
     */
    public BitwigApiFacade getFacade() {
        return facade;
    }

    /**
     * Get the MCP server manager.
     */
    public McpServerManager getMcpServer() {
        return mcpServer;
    }
}
