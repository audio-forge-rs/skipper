package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;
import java.util.Map;

/**
 * MCP tool to get comprehensive project snapshot in one call.
 *
 * Returns transport state, all tracks with their devices,
 * Skipper presence, and instrument info. Designed to minimize
 * token usage by eliminating multiple round-trip calls.
 */
public class GetProjectSnapshotTool {

    private static final String EMPTY_SCHEMA = """
        {"type": "object", "properties": {}, "required": []}
        """;
    private static final ObjectMapper mapper = new ObjectMapper();

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("get_project_snapshot",
                "Get full project state: transport, tracks, devices, Skipper status",
                EMPTY_SCHEMA),
            (exchange, args) -> {
                try {
                    Map<String, Object> snapshot = facade.getProjectSnapshot();
                    String json = mapper.writeValueAsString(snapshot);
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent(json)),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: get_project_snapshot error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }
}
