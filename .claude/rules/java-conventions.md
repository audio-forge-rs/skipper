---
paths: gilligan/**/*.java
---

# Java Conventions for Gilligan

## MCP Tool Pattern

```java
public class MyTool {
    public static McpServerFeatures.SyncToolSpecification create(
            BitwigApiFacade facade, ControllerHost host) {

        McpSchema.Tool tool = McpSchema.Tool.builder()
            .name("tool_name")
            .description("Brief description (~10 words)")
            .inputSchema(McpSchema.EMPTY_OBJECT_SCHEMA)
            .build();

        return McpServerFeatures.SyncToolSpecification.builder()
            .tool(tool)
            .handler((exchange, request) -> {
                // Implementation
                return new McpSchema.CallToolResult(
                    List.of(new McpSchema.TextContent("Result")),
                    false  // isError
                );
            })
            .build();
    }
}
```

## Key Dependencies

```xml
<!-- MCP SDK (via BOM) -->
<dependency>
    <groupId>io.modelcontextprotocol.sdk</groupId>
    <artifactId>mcp</artifactId>
</dependency>

<!-- Jetty for HTTP server -->
<dependency>
    <groupId>org.eclipse.jetty</groupId>
    <artifactId>jetty-server</artifactId>
    <version>11.0.20</version>
</dependency>

<!-- Bitwig Controller API - PROVIDED by Bitwig runtime -->
<dependency>
    <groupId>com.bitwig</groupId>
    <artifactId>extension-api</artifactId>
    <version>19</version>
    <scope>provided</scope>
</dependency>
```

## Extension Requirements

**Java SPI Service File Required:**
`src/main/resources/META-INF/services/com.bitwig.extension.ExtensionDefinition`

```
com.bedwards.gilligan.GilliganExtensionDefinition
```

**Bitwig API must be `provided` scope** - NOT bundled in JAR.

## MCP Token Optimization

Following Anthropic's MCP best practices:
- Minimal tool set: Only essential operations
- Concise descriptions: ~10 words per tool
- Filtered responses: Return only essential data
- Progressive disclosure ready: Can add `search_tools` meta-tool if needed

## Network Access

Java extensions have **full network access**:
- HTTP servers (Jetty) - what we use for MCP
- WebSocket, OSC (UDP), raw TCP/UDP
- Any Java networking library

**Thread safety warning:** API calls from outside Control Surface thread may be unsafe.

**Port binding:** Ensure proper `stop()` with `jettyServer.join()` before restart to release port.
