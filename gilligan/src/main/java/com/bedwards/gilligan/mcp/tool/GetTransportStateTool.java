package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;
import java.util.Map;

/**
 * MCP tool to get Bitwig transport state.
 * Returns: tempo, time signature, position, playing/recording status.
 */
public class GetTransportStateTool {

    private static final String EMPTY_SCHEMA = """
        {"type": "object", "properties": {}, "required": []}
        """;
    private static final ObjectMapper mapper = new ObjectMapper();

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("get_transport", "Get Bitwig transport (tempo, time sig, status)", EMPTY_SCHEMA),
            (exchange, args) -> {
                try {
                    Map<String, Object> state = facade.getTransportState();
                    String json = mapper.writeValueAsString(state);
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent(json)),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: get_transport error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }
}
