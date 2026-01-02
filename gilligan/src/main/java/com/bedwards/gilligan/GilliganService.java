package com.bedwards.gilligan;

import com.bitwig.extension.controller.api.ControllerHost;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Shared service layer for Gilligan commands.
 *
 * Used by both MCP tools and REST API to avoid code duplication.
 * All methods return a Result record with success/error status and data.
 */
public class GilliganService {

    private static final ObjectMapper mapper = new ObjectMapper();

    // Valid bar lengths for program staging
    private static final double[] VALID_BAR_LENGTHS = {
        0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0
    };

    private final BitwigApiFacade facade;
    private final ControllerHost host;

    public GilliganService(BitwigApiFacade facade, ControllerHost host) {
        this.facade = facade;
        this.host = host;
    }

    /**
     * Result of a service operation.
     */
    public record Result(boolean success, Object data, String error) {
        public static Result ok(Object data) {
            return new Result(true, data, null);
        }

        public static Result ok(String message) {
            return new Result(true, Map.of("message", message), null);
        }

        public static Result err(String error) {
            return new Result(false, null, error);
        }

        public String toJson() {
            try {
                if (success) {
                    return mapper.writeValueAsString(data);
                } else {
                    return mapper.writeValueAsString(Map.of("error", error));
                }
            } catch (Exception e) {
                return "{\"error\": \"JSON serialization failed\"}";
            }
        }
    }

    // ========== Transport Commands ==========

    public Result play() {
        try {
            facade.play();
            return Result.ok("Playback started");
        } catch (Exception e) {
            host.errorln("Gilligan: play error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    public Result stop() {
        try {
            facade.stop();
            return Result.ok("Playback stopped");
        } catch (Exception e) {
            host.errorln("Gilligan: stop error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    public Result record() {
        try {
            facade.record();
            return Result.ok("Recording toggled");
        } catch (Exception e) {
            host.errorln("Gilligan: record error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    public Result getTransport() {
        try {
            Map<String, Object> state = facade.getTransportState();
            return Result.ok(state);
        } catch (Exception e) {
            host.errorln("Gilligan: getTransport error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    // ========== Track Commands ==========

    public Result listTracks() {
        try {
            List<Map<String, Object>> tracks = facade.getAllTracks();
            return Result.ok(tracks);
        } catch (Exception e) {
            host.errorln("Gilligan: listTracks error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    public Result getSelectedTrack() {
        try {
            Map<String, Object> track = facade.getSelectedTrack();
            return Result.ok(track);
        } catch (Exception e) {
            host.errorln("Gilligan: getSelectedTrack error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    public Result createTrack(String type) {
        try {
            if ("instrument".equalsIgnoreCase(type)) {
                facade.createInstrumentTrack();
                return Result.ok("Instrument track created");
            } else if ("audio".equalsIgnoreCase(type)) {
                facade.createAudioTrack();
                return Result.ok("Audio track created");
            } else {
                return Result.err("Invalid track type: " + type + ". Use 'instrument' or 'audio'");
            }
        } catch (Exception e) {
            host.errorln("Gilligan: createTrack error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    public Result renameTrack(String name) {
        try {
            if (name == null || name.isEmpty()) {
                return Result.err("Track name required");
            }
            facade.renameSelectedTrack(name);
            return Result.ok("Track renamed to: " + name);
        } catch (Exception e) {
            host.errorln("Gilligan: renameTrack error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    // ========== Device Commands ==========

    public Result getSelectedDevice() {
        try {
            Map<String, Object> device = facade.getSelectedDevice();
            return Result.ok(device);
        } catch (Exception e) {
            host.errorln("Gilligan: getSelectedDevice error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    // ========== Workflow Commands ==========

    public Result getProjectSnapshot() {
        try {
            Map<String, Object> snapshot = facade.getProjectSnapshot();
            return Result.ok(snapshot);
        } catch (Exception e) {
            host.errorln("Gilligan: getProjectSnapshot error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    @SuppressWarnings("unchecked")
    public Result stagePrograms(Map<String, Object> args) {
        try {
            List<Map<String, Object>> stages = (List<Map<String, Object>>) args.get("stages");
            String commitAt = (String) args.getOrDefault("commitAt", "next_bar");

            if (stages == null || stages.isEmpty()) {
                return Result.err("No stages provided");
            }

            // Validate bar lengths
            for (Map<String, Object> stage : stages) {
                Map<String, Object> program = (Map<String, Object>) stage.get("program");
                if (program != null) {
                    Number lengthBars = (Number) program.get("lengthBars");
                    if (lengthBars != null && !isValidBarLength(lengthBars.doubleValue())) {
                        return Result.err("Invalid bar length " + lengthBars +
                            ". Must be power-of-2: 0.125, 0.25, 0.5, 1, 2, 4, 8, 16");
                    }
                }
            }

            // TODO: Implement actual staging via Skipper plugin communication
            Map<String, Object> result = new HashMap<>();
            result.put("staged", stages.size());
            result.put("commitAt", commitAt);
            result.put("status", "pending_implementation");

            StringBuilder tracks = new StringBuilder();
            for (Map<String, Object> stage : stages) {
                if (tracks.length() > 0) tracks.append(", ");
                tracks.append(stage.get("track"));
            }
            result.put("tracks", tracks.toString());

            host.println("Gilligan: stage_programs called with " + stages.size() + " stages");

            return Result.ok(result);
        } catch (Exception e) {
            host.errorln("Gilligan: stagePrograms error: " + e.getMessage());
            return Result.err(e.getMessage());
        }
    }

    private boolean isValidBarLength(double length) {
        for (double valid : VALID_BAR_LENGTHS) {
            if (Math.abs(length - valid) < 0.001) {
                return true;
            }
        }
        return false;
    }

    // ========== Command Dispatcher ==========

    /**
     * Dispatch a command by name with optional arguments.
     * Used by REST API for simple routing.
     */
    @SuppressWarnings("unchecked")
    public Result dispatch(String command, Map<String, Object> args) {
        if (args == null) {
            args = Map.of();
        }

        return switch (command) {
            case "play", "transport_play" -> play();
            case "stop", "transport_stop" -> stop();
            case "record", "transport_record" -> record();
            case "transport", "get_transport" -> getTransport();
            case "tracks", "list_tracks" -> listTracks();
            case "track", "get_selected_track" -> getSelectedTrack();
            case "create_track" -> createTrack((String) args.get("type"));
            case "rename_track" -> renameTrack((String) args.get("name"));
            case "device", "get_selected_device" -> getSelectedDevice();
            case "snapshot", "get_project_snapshot" -> getProjectSnapshot();
            case "stage", "stage_programs" -> stagePrograms(args);
            default -> Result.err("Unknown command: " + command);
        };
    }
}
