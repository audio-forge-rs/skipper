package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;
import java.util.Map;

/**
 * MCP tool to create a new track in Bitwig.
 */
public class CreateTrackTool {

    private static final String SCHEMA = """
        {
            "type": "object",
            "properties": {
                "type": {
                    "type": "string",
                    "enum": ["instrument", "audio"],
                    "description": "Type of track to create: 'instrument' or 'audio'"
                }
            },
            "required": ["type"]
        }
        """;

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("create_track", "Create a new track in Bitwig", SCHEMA),
            (exchange, args) -> {
                try {
                    @SuppressWarnings("unchecked")
                    Map<String, Object> arguments = (Map<String, Object>) args;
                    String type = (String) arguments.get("type");

                    if ("instrument".equals(type)) {
                        facade.createInstrumentTrack();
                    } else if ("audio".equals(type)) {
                        facade.createAudioTrack();
                    } else {
                        return new McpSchema.CallToolResult(
                            List.of(new McpSchema.TextContent("Invalid track type: " + type)),
                            true
                        );
                    }

                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Created " + type + " track")),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: create_track error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }
}
