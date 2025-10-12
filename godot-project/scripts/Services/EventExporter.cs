using Godot;
using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using Outpost3.Core.Events;

namespace Outpost3.Services;

/// <summary>
/// Static service for exporting game events to various file formats.
/// Supports JSON and YAML export.
/// </summary>
public static class EventExporter
{
    /// <summary>
    /// Exports events to a JSON file.
    /// </summary>
    /// <param name="events">The events to export.</param>
    /// <param name="filePath">The full path where the file should be saved.</param>
    /// <exception cref="IOException">Thrown when file operations fail.</exception>
    public static void ExportToJson(IEnumerable<GameEvent> events, string filePath)
    {
        try
        {
            var eventList = events as List<GameEvent> ?? new List<GameEvent>(events);

            var options = new JsonSerializerOptions
            {
                WriteIndented = true,
                PropertyNamingPolicy = JsonNamingPolicy.CamelCase
            };

            var json = JsonSerializer.Serialize(eventList, options);
            File.WriteAllText(filePath, json);

            // Only use GD.Print if running in Godot context
            if (Engine.IsEditorHint() || OS.HasFeature("standalone"))
            {
                GD.Print($"EventExporter: Successfully exported {eventList.Count} events to {filePath}");
            }
        }
        catch (Exception ex)
        {
            // Only use GD.PrintErr if running in Godot context
            if (Engine.IsEditorHint() || OS.HasFeature("standalone"))
            {
                GD.PrintErr($"EventExporter: Failed to export to JSON: {ex.Message}");
            }
            throw new IOException($"Failed to export events to JSON file: {filePath}", ex);
        }
    }

    /// <summary>
    /// Exports events to a YAML file.
    /// Note: This method requires the YamlDotNet NuGet package.
    /// </summary>
    /// <param name="events">The events to export.</param>
    /// <param name="filePath">The full path where the file should be saved.</param>
    /// <exception cref="IOException">Thrown when file operations fail.</exception>
    /// <exception cref="NotImplementedException">Thrown if YamlDotNet is not available.</exception>
    public static void ExportToYaml(IEnumerable<GameEvent> events, string filePath)
    {
        try
        {
            // TODO: Add YamlDotNet NuGet package to project
            // For now, this is a placeholder implementation
            
            // When YamlDotNet is available, use this pattern:
            // var eventList = events as List<GameEvent> ?? new List<GameEvent>(events);
            // var serializer = new SerializerBuilder()
            //     .WithNamingConvention(CamelCaseNamingConvention.Instance)
            //     .Build();
            // var yaml = serializer.Serialize(eventList);
            // File.WriteAllText(filePath, yaml);
            // GD.Print($"EventExporter: Successfully exported {eventList.Count} events to {filePath}");

            throw new NotImplementedException(
                "YAML export requires YamlDotNet NuGet package. " +
                "Add the package with: dotnet add package YamlDotNet");
        }
        catch (NotImplementedException)
        {
            throw;
        }
        catch (Exception ex)
        {
            GD.PrintErr($"EventExporter: Failed to export to YAML: {ex.Message}");
            throw new IOException($"Failed to export events to YAML file: {filePath}", ex);
        }
    }

    /// <summary>
    /// Generates a timestamp-based filename for exports.
    /// </summary>
    /// <param name="prefix">Prefix for the filename (e.g., "event_log").</param>
    /// <param name="format">File format extension (e.g., "json", "yaml").</param>
    /// <returns>A filename in the format: prefix_YYYYMMDD_HHMMSS.format</returns>
    public static string GenerateExportFilename(string prefix, string format)
    {
        var timestamp = DateTime.Now.ToString("yyyyMMdd_HHmmss");
        return $"{prefix}_{timestamp}.{format}";
    }
}
