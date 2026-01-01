package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;
import java.util.Map;

/**
 * MCP tool to get currently selected device.
 */
public class GetSelectedDeviceTool {

    private static final String EMPTY_SCHEMA = """
        {"type": "object", "properties": {}, "required": []}
        """;
    private static final ObjectMapper mapper = new ObjectMapper();

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("get_selected_device", "Get selected Bitwig device info", EMPTY_SCHEMA),
            (exchange, args) -> {
                try {
                    Map<String, Object> device = facade.getSelectedDevice();
                    String json = mapper.writeValueAsString(device);
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent(json)),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: get_selected_device error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }
}
