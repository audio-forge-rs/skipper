package com.bedwards.gilligan.mcp.tool;

import com.bedwards.gilligan.BitwigApiFacade;
import com.bitwig.extension.controller.api.ControllerHost;
import com.fasterxml.jackson.databind.ObjectMapper;
import io.modelcontextprotocol.server.McpServerFeatures;
import io.modelcontextprotocol.spec.McpSchema;

import java.util.List;
import java.util.Map;

/**
 * MCP tool to stage programs on multiple tracks for beat-synchronized commit.
 *
 * Program length constraint: Must be power-of-2 bar multiples/divisors.
 * Valid: 1/4, 1/2, 1, 2, 4, 8 bars
 * Invalid: 1.5, 3, 7/8 bars
 *
 * Staged programs are held in Skipper plugin instances until commit.
 */
public class StageProgramsTool {

    private static final String SCHEMA = """
        {
            "type": "object",
            "properties": {
                "stages": {
                    "type": "array",
                    "description": "Array of track/program pairs to stage",
                    "items": {
                        "type": "object",
                        "properties": {
                            "track": {"type": "string", "description": "Track name"},
                            "program": {
                                "type": "object",
                                "properties": {
                                    "lengthBars": {"type": "number", "description": "Program length (power-of-2: 0.25, 0.5, 1, 2, 4...)"},
                                    "notes": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "pitch": {"type": "integer", "description": "MIDI pitch 0-127"},
                                                "startBeat": {"type": "number", "description": "Start position in beats"},
                                                "lengthBeats": {"type": "number", "description": "Note duration in beats"},
                                                "velocity": {"type": "number", "description": "Velocity 0-1"}
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        "required": ["track", "program"]
                    }
                },
                "commitAt": {
                    "type": "string",
                    "description": "When to commit: 'immediate', 'next_bar', 'next_beat', or beat number",
                    "default": "next_bar"
                }
            },
            "required": ["stages"]
        }
        """;

    private static final ObjectMapper mapper = new ObjectMapper();

    // Valid bar lengths: powers of 2 from 1/8 to 16 bars
    private static final double[] VALID_BAR_LENGTHS = {
        0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0
    };

    public static McpServerFeatures.SyncToolSpecification create(BitwigApiFacade facade, ControllerHost host) {
        return new McpServerFeatures.SyncToolSpecification(
            new McpSchema.Tool("stage_programs",
                "Stage MIDI programs on tracks for beat-synced commit",
                SCHEMA),
            (exchange, args) -> {
                try {
                    @SuppressWarnings("unchecked")
                    List<Map<String, Object>> stages = (List<Map<String, Object>>) args.get("stages");
                    String commitAt = (String) args.getOrDefault("commitAt", "next_bar");

                    if (stages == null || stages.isEmpty()) {
                        return new McpSchema.CallToolResult(
                            List.of(new McpSchema.TextContent("Error: No stages provided")),
                            true
                        );
                    }

                    // Validate bar lengths
                    for (Map<String, Object> stage : stages) {
                        @SuppressWarnings("unchecked")
                        Map<String, Object> program = (Map<String, Object>) stage.get("program");
                        if (program != null) {
                            Number lengthBars = (Number) program.get("lengthBars");
                            if (lengthBars != null && !isValidBarLength(lengthBars.doubleValue())) {
                                return new McpSchema.CallToolResult(
                                    List.of(new McpSchema.TextContent(
                                        "Error: Invalid bar length " + lengthBars +
                                        ". Must be power-of-2: 0.125, 0.25, 0.5, 1, 2, 4, 8, 16")),
                                    true
                                );
                            }
                        }
                    }

                    // TODO: Implement actual staging via Skipper plugin communication
                    // For now, return acknowledgment that staging request was received
                    StringBuilder result = new StringBuilder();
                    result.append("Staging ").append(stages.size()).append(" program(s)");
                    result.append(" for commit at ").append(commitAt);
                    result.append("\n\nTracks:");
                    for (Map<String, Object> stage : stages) {
                        result.append("\n  - ").append(stage.get("track"));
                    }
                    result.append("\n\n[Note: Full Skipper plugin integration pending]");

                    host.println("Gilligan MCP: stage_programs called with " + stages.size() + " stages");

                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent(result.toString())),
                        false
                    );
                } catch (Exception e) {
                    host.errorln("Gilligan MCP: stage_programs error: " + e.getMessage());
                    return new McpSchema.CallToolResult(
                        List.of(new McpSchema.TextContent("Error: " + e.getMessage())),
                        true
                    );
                }
            }
        );
    }

    private static boolean isValidBarLength(double length) {
        for (double valid : VALID_BAR_LENGTHS) {
            if (Math.abs(length - valid) < 0.001) {
                return true;
            }
        }
        return false;
    }
}
