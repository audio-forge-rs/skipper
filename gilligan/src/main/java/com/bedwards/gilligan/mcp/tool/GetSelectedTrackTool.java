package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;
import java.util.Map;

/**
 * MCP tool to get currently selected track.
 */
public class GetSelectedTrackTool {

    private static final String EMPTY_SCHEMA = """
        {"type": "object", "properties": {}, "required": []}
        """;
    private static final ObjectMapper mapper = new ObjectMapper();

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("get_selected_track", "Get selected Bitwig track info", EMPTY_SCHEMA),
            (exchange, args) -> {
                try {
                    Map<String, Object> track = facade.getSelectedTrack();
                    String json = mapper.writeValueAsString(track);
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent(json)),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: get_selected_track error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }
}
