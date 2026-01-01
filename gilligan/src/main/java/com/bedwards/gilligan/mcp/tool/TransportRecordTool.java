package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;

/**
 * MCP tool to toggle Bitwig recording.
 */
public class TransportRecordTool {

    private static final String EMPTY_SCHEMA = """
        {"type": "object", "properties": {}, "required": []}
        """;

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("transport_record", "Toggle Bitwig recording", EMPTY_SCHEMA),
            (exchange, args) -> {
                try {
                    facade.record();
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Recording toggled")),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: transport_record error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }
}
