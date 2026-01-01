package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;

/**
 * MCP tool to start Bitwig transport playback.
 * Minimal design for token efficiency.
 */
public class TransportPlayTool {

    private static final String EMPTY_SCHEMA = """
        {"type": "object", "properties": {}, "required": []}
        """;

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("transport_play", "Start Bitwig playback", EMPTY_SCHEMA),
            (exchange, args) -> {
                try {
                    facade.play();
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Playback started")),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: transport_play error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }
}
