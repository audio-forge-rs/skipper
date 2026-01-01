package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;
import java.util.Map;

/**
 * MCP tool to list Bitwig tracks.
 * Returns minimal info: name, color, position, type.
 */
public class ListTracksTool {

    private static final String EMPTY_SCHEMA = """
        {"type": "object", "properties": {}, "required": []}
        """;
    private static final ObjectMapper mapper = new ObjectMapper();

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("list_tracks", "List Bitwig tracks (name, color, type)", EMPTY_SCHEMA),
            (exchange, args) -> {
                try {
                    List<Map<String, Object>> tracks = facade.getAllTracks();
                    String json = mapper.writeValueAsString(tracks);
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent(json)),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: list_tracks error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }
}
