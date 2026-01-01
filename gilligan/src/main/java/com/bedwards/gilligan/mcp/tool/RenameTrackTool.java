package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;
import java.util.Map;

/**
 * MCP tool to rename the currently selected track in Bitwig.
 */
public class RenameTrackTool {

    private static final String SCHEMA = """
        {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The new name for the currently selected track"
                }
            },
            "required": ["name"]
        }
        """;

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("rename_track", "Rename the currently selected track", SCHEMA),
            (exchange, args) -> {
                try {
                    @SuppressWarnings("unchecked")
                    Map<String, Object> arguments = (Map<String, Object>) args;
                    String name = (String) arguments.get("name");

                    if (name == null || name.isEmpty()) {
                        return new McpSchema.CallToolResult(
                            List.of(new McpSchema.TextContent("Error: name cannot be empty")),
                            true
                        );
                    }

                    facade.renameSelectedTrack(name);

                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Renamed track to: " + name)),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: rename_track error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }
}
