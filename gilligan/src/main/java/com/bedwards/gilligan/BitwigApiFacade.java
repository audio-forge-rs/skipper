package com.bedwards.gilligan;

import com.bitwig.extension.controller.api.*;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Facade for Bitwig Controller API
 *
 * Provides a simplified interface to Bitwig's APIs for use by MCP tools.
 * Caches API objects since they can only be created once during extension init.
 */
public class BitwigApiFacade {

    private final ControllerHost host;
    private final Transport transport;
    private final CursorTrack cursorTrack;
    private final CursorDevice cursorDevice;
    private final TrackBank trackBank;
    private final Application application;

    // Cached state (updated by observers)
    private double tempo = 120.0;
    private int timeSignatureNumerator = 4;
    private int timeSignatureDenominator = 4;
    private double positionBeats = 0.0;
    private boolean isPlaying = false;
    private boolean isRecording = false;
    private boolean isLoopActive = false;

    private String currentTrackName = "";
    private String currentTrackColor = "#808080";
    private boolean isTrackGroup = false;
    private int trackPosition = -1;

    private String currentDeviceName = "";
    private boolean deviceExists = false;

    // Track bank info
    private final List<TrackInfo> tracks = new ArrayList<>();

    public BitwigApiFacade(ControllerHost host) {
        this.host = host;

        // Create API objects
        application = host.createApplication();
        transport = host.createTransport();

        // Cursor track (follows user selection)
        cursorTrack = host.createCursorTrack("GilliganCursor", "Gilligan Track", 0, 0, true);

        // Cursor device (follows user selection)
        cursorDevice = cursorTrack.createCursorDevice("GilliganDevice", "Gilligan Device", 0,
            CursorDeviceFollowMode.FOLLOW_SELECTION);

        // Track bank for accessing all tracks (16 tracks)
        trackBank = host.createTrackBank(16, 0, 0, true);

        // Set up observers
        setupTransportObservers();
        setupTrackObservers();
        setupDeviceObservers();
        setupTrackBankObservers();
    }

    private void setupTransportObservers() {
        transport.isPlaying().addValueObserver(playing -> isPlaying = playing);
        transport.isArrangerRecordEnabled().addValueObserver(recording -> isRecording = recording);
        transport.isArrangerLoopEnabled().addValueObserver(looping -> isLoopActive = looping);
        transport.tempo().value().addValueObserver(t -> tempo = t);
        transport.getPosition().addValueObserver(pos -> positionBeats = pos);
        transport.timeSignature().numerator().addValueObserver(num -> timeSignatureNumerator = num);
        transport.timeSignature().denominator().addValueObserver(den -> timeSignatureDenominator = den);

        // Mark as interested
        transport.tempo().markInterested();
        transport.getPosition().markInterested();
        transport.isPlaying().markInterested();
        transport.isArrangerRecordEnabled().markInterested();
        transport.isArrangerLoopEnabled().markInterested();
        transport.timeSignature().numerator().markInterested();
        transport.timeSignature().denominator().markInterested();
    }

    private void setupTrackObservers() {
        cursorTrack.name().addValueObserver(name -> currentTrackName = name);
        cursorTrack.color().addValueObserver((r, g, b) -> {
            currentTrackColor = String.format("#%02X%02X%02X",
                (int)(r * 255), (int)(g * 255), (int)(b * 255));
        });
        cursorTrack.position().addValueObserver(pos -> trackPosition = pos);
        cursorTrack.isGroup().addValueObserver(isGroup -> isTrackGroup = isGroup);

        cursorTrack.name().markInterested();
        cursorTrack.color().markInterested();
        cursorTrack.position().markInterested();
        cursorTrack.isGroup().markInterested();
    }

    private void setupDeviceObservers() {
        cursorDevice.name().addValueObserver(name -> currentDeviceName = name);
        cursorDevice.exists().addValueObserver(exists -> deviceExists = exists);

        cursorDevice.name().markInterested();
        cursorDevice.exists().markInterested();
    }

    private void setupTrackBankObservers() {
        // Initialize track info list
        for (int i = 0; i < 16; i++) {
            tracks.add(new TrackInfo());
        }

        // Set up observers for each track in the bank
        for (int i = 0; i < 16; i++) {
            Track track = trackBank.getItemAt(i);
            final int index = i;

            track.name().addValueObserver(name -> tracks.get(index).name = name);
            track.color().addValueObserver((r, g, b) -> {
                tracks.get(index).color = String.format("#%02X%02X%02X",
                    (int)(r * 255), (int)(g * 255), (int)(b * 255));
            });
            track.position().addValueObserver(pos -> tracks.get(index).position = pos);
            track.isGroup().addValueObserver(isGroup -> tracks.get(index).isGroup = isGroup);
            track.exists().addValueObserver(exists -> tracks.get(index).exists = exists);
            track.trackType().addValueObserver(type -> tracks.get(index).trackType = type);

            track.name().markInterested();
            track.color().markInterested();
            track.position().markInterested();
            track.isGroup().markInterested();
            track.exists().markInterested();
            track.trackType().markInterested();
        }
    }

    // Transport control methods
    public void play() {
        transport.play();
    }

    public void stop() {
        transport.stop();
    }

    public void record() {
        transport.record();
    }

    public void toggleLoop() {
        transport.isArrangerLoopEnabled().toggle();
    }

    // State getters
    public Map<String, Object> getTransportState() {
        Map<String, Object> state = new HashMap<>();
        state.put("playing", isPlaying);
        state.put("recording", isRecording);
        state.put("loopActive", isLoopActive);
        state.put("tempo", tempo);
        state.put("positionBeats", positionBeats);
        state.put("timeSignature", timeSignatureNumerator + "/" + timeSignatureDenominator);
        state.put("timeSignatureNumerator", timeSignatureNumerator);
        state.put("timeSignatureDenominator", timeSignatureDenominator);
        return state;
    }

    public Map<String, Object> getSelectedTrack() {
        Map<String, Object> track = new HashMap<>();
        track.put("name", currentTrackName);
        track.put("color", currentTrackColor);
        track.put("position", trackPosition);
        track.put("isGroup", isTrackGroup);
        return track;
    }

    public Map<String, Object> getSelectedDevice() {
        Map<String, Object> device = new HashMap<>();
        device.put("exists", deviceExists);
        device.put("name", currentDeviceName);
        return device;
    }

    public List<Map<String, Object>> getAllTracks() {
        List<Map<String, Object>> result = new ArrayList<>();
        for (TrackInfo info : tracks) {
            if (info.exists) {
                Map<String, Object> track = new HashMap<>();
                track.put("name", info.name);
                track.put("color", info.color);
                track.put("position", info.position);
                track.put("isGroup", info.isGroup);
                track.put("trackType", info.trackType);
                result.add(track);
            }
        }
        return result;
    }

    public String getHostVersion() {
        return host.getHostVersion();
    }

    public int getHostApiVersion() {
        return host.getHostApiVersion();
    }

    // Track manipulation methods

    /**
     * Create a new instrument track at the end of the arrangement.
     */
    public void createInstrumentTrack() {
        application.createInstrumentTrack(-1);
    }

    /**
     * Create a new audio track at the end of the arrangement.
     */
    public void createAudioTrack() {
        application.createAudioTrack(-1);
    }

    /**
     * Rename the currently selected track.
     * @param name The new name for the track
     */
    public void renameSelectedTrack(String name) {
        cursorTrack.name().set(name);
    }

    /**
     * Insert a device/plugin on the currently selected track by browsing for it.
     * This opens the browser and filters by the given query.
     * @param query The search query for the device/plugin
     */
    public void browseToInsertDevice(String query) {
        // Use the cursor device's browser to insert after current device
        cursorDevice.browseToInsertAfterDevice();
    }

    /**
     * Select a specific device on the current track by name.
     * @param deviceName The name of the device to select
     */
    public void selectDeviceByName(String deviceName) {
        // Navigate through devices to find matching name
        // For now, this is limited - would need device bank for full navigation
        host.println("Gilligan: selectDeviceByName not fully implemented yet");
    }

    // Track info container
    private static class TrackInfo {
        String name = "";
        String color = "#808080";
        int position = -1;
        boolean isGroup = false;
        boolean exists = false;
        String trackType = "";
    }
}
