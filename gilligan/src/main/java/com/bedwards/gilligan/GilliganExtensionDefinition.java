package com.bedwards.gilligan;

import java.util.UUID;

import com.bitwig.extension.api.PlatformType;
import com.bitwig.extension.controller.AutoDetectionMidiPortNamesList;
import com.bitwig.extension.controller.ControllerExtensionDefinition;
import com.bitwig.extension.controller.api.ControllerHost;

/**
 * Gilligan Extension Definition
 *
 * Bitwig Controller Extension that displays DAW and track information.
 * Companion to the Skipper CLAP plugin - provides matching info via Controller API.
 */
public class GilliganExtensionDefinition extends ControllerExtensionDefinition {

    private static final UUID DRIVER_ID = UUID.fromString("a1b2c3d4-e5f6-7890-abcd-ef1234567890");

    @Override
    public String getName() {
        return "Gilligan";
    }

    @Override
    public String getAuthor() {
        return "bedwards";
    }

    @Override
    public String getVersion() {
        return "0.1.0";
    }

    @Override
    public UUID getId() {
        return DRIVER_ID;
    }

    @Override
    public String getHardwareVendor() {
        return "bedwards";
    }

    @Override
    public String getHardwareModel() {
        return "Gilligan Info Display";
    }

    @Override
    public int getRequiredAPIVersion() {
        return 19;
    }

    @Override
    public int getNumMidiInPorts() {
        return 0;  // No MIDI required - this is a virtual controller
    }

    @Override
    public int getNumMidiOutPorts() {
        return 0;
    }

    @Override
    public void listAutoDetectionMidiPortNames(
            AutoDetectionMidiPortNamesList list,
            PlatformType platformType) {
        // No auto-detection - manually added controller
    }

    @Override
    public GilliganExtension createInstance(ControllerHost host) {
        return new GilliganExtension(this, host);
    }

    @Override
    public String getHelpFilePath() {
        return "Documentation/Gilligan.html";
    }

    @Override
    public boolean shouldFailOnDeprecatedUse() {
        return true;
    }

    @Override
    public boolean isUsingBetaAPI() {
        return false;
    }
}
