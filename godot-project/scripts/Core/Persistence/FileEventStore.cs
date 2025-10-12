using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;
using Outpost3.Core.Events;

namespace Outpost3.Core.Persistence;

/// <summary>
/// File-based event store using JSON Lines format.
/// Each event is stored as one line: offset|gameTime|eventType|jsonPayload
/// </summary>
/// <remarks>
/// Thread-Safety: Single-writer, multiple-readers pattern.
/// File Format: JSON Lines (one event per line, pipe-delimited fields)
/// </remarks>
public class FileEventStore : IEventStore
{
    private readonly string _filePath;
    private readonly object _writeLock = new();
    private readonly JsonSerializerOptions _jsonOptions;
    private long _currentOffset = -1;

    /// <summary>
    /// Creates a new FileEventStore with the specified file path.
    /// </summary>
    /// <param name="filePath">The path to the event log file.</param>
    /// <exception cref="ArgumentNullException">Thrown if filePath is null or empty.</exception>
    public FileEventStore(string filePath)
    {
        if (string.IsNullOrWhiteSpace(filePath))
        {
            throw new ArgumentNullException(nameof(filePath));
        }

        _filePath = filePath;

        // Initialize JSON serialization options
        _jsonOptions = new JsonSerializerOptions
        {
            WriteIndented = false,
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            Converters =
            {
                new JsonStringEnumConverter(JsonNamingPolicy.CamelCase),
                new GameEventJsonConverter()
            }
        };

        // If file exists, count lines to restore current offset
        if (File.Exists(_filePath))
        {
            try
            {
                var lineCount = File.ReadLines(_filePath).Count();
                _currentOffset = lineCount - 1;
            }
            catch (IOException ex)
            {
                throw new EventStoreException(
                    $"Failed to read existing event store file: {_filePath}", ex);
            }
        }
    }

    /// <inheritdoc/>
    public long Append(params GameEvent[] events)
    {
        if (events == null)
        {
            throw new ArgumentNullException(nameof(events));
        }

        if (events.Length == 0)
        {
            throw new ArgumentException("Cannot append empty event array.", nameof(events));
        }

        lock (_writeLock)
        {
            var startingOffset = _currentOffset + 1;

            try
            {
                // Ensure directory exists
                var directory = Path.GetDirectoryName(_filePath);
                if (!string.IsNullOrEmpty(directory) && !Directory.Exists(directory))
                {
                    Directory.CreateDirectory(directory);
                }

                // Open file for appending
                using var stream = new FileStream(_filePath, FileMode.Append, FileAccess.Write, FileShare.Read);
                using var writer = new StreamWriter(stream);

                foreach (var evt in events)
                {
                    _currentOffset++;

                    // Enrich event with offset
                    var enrichedEvent = evt with { Offset = _currentOffset };

                    // Serialize to JSON Lines format: offset|gameTime|eventType|jsonPayload
                    var json = JsonSerializer.Serialize(enrichedEvent, enrichedEvent.GetType(), _jsonOptions);
                    var line = $"{enrichedEvent.Offset}|{enrichedEvent.GameTime}|{enrichedEvent.EventType}|{json}";

                    writer.WriteLine(line);
                }

                // Flush to ensure data is written immediately
                writer.Flush();
                stream.Flush(flushToDisk: true);
            }
            catch (IOException ex) when (IsOutOfDiskSpace(ex))
            {
                throw new EventStoreException(
                    "Failed to append events: disk full or quota exceeded.", ex);
            }
            catch (IOException ex)
            {
                throw new EventStoreException(
                    "Failed to append events due to I/O error.", ex);
            }
            catch (Exception ex)
            {
                throw new EventStoreException(
                    "Failed to append events due to unexpected error.", ex);
            }

            return startingOffset;
        }
    }

    /// <inheritdoc/>
    public IEnumerable<GameEvent> ReadFrom(long offset = 0)
    {
        if (offset < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(offset), "Offset cannot be negative.");
        }

        // If file doesn't exist or offset is beyond current, return empty
        if (!File.Exists(_filePath) || offset > _currentOffset)
        {
            yield break;
        }

        StreamReader reader = null;
        try
        {
            reader = new StreamReader(
                new FileStream(_filePath, FileMode.Open, FileAccess.Read, FileShare.ReadWrite));

            long currentLine = 0;

            // Skip lines until we reach the requested offset
            while (currentLine < offset && !reader.EndOfStream)
            {
                reader.ReadLine();
                currentLine++;
            }

            // Read and yield events from the offset onwards
            while (!reader.EndOfStream)
            {
                var line = reader.ReadLine();
                if (string.IsNullOrWhiteSpace(line))
                {
                    currentLine++;
                    continue;
                }

                GameEvent evt;
                try
                {
                    evt = ParseEventLine(line, currentLine);
                }
                catch (Exception ex)
                {
                    throw new EventStoreException(
                        $"Corrupted event at line {currentLine}", currentLine, ex);
                }

                yield return evt;
                currentLine++;
            }
        }
        finally
        {
            if (reader != null)
            {
                reader.Dispose();
            }
        }
    }

    /// <inheritdoc/>
    public long CurrentOffset => _currentOffset;

    /// <inheritdoc/>
    public long Count
    {
        get
        {
            if (!File.Exists(_filePath))
            {
                return 0;
            }

            try
            {
                return File.ReadLines(_filePath).Count();
            }
            catch (IOException)
            {
                // If we can't read the file, return 0
                return 0;
            }
        }
    }

    /// <summary>
    /// Parses a single line from the event log file.
    /// </summary>
    /// <param name="line">The line to parse in format: offset|gameTime|eventType|jsonPayload</param>
    /// <param name="lineNumber">The line number (for error reporting).</param>
    /// <returns>The parsed GameEvent.</returns>
    /// <exception cref="FormatException">Thrown if the line format is invalid.</exception>
    private GameEvent ParseEventLine(string line, long lineNumber)
    {
        // Split by pipe character: offset|gameTime|eventType|jsonPayload
        var parts = line.Split('|', 4);
        if (parts.Length != 4)
        {
            throw new FormatException(
                $"Invalid line format at line {lineNumber}. Expected 4 parts, got {parts.Length}.");
        }

        if (!long.TryParse(parts[0], out var offset))
        {
            throw new FormatException($"Invalid offset at line {lineNumber}: {parts[0]}");
        }

        if (!float.TryParse(parts[1], out var gameTime))
        {
            throw new FormatException($"Invalid game time at line {lineNumber}: {parts[1]}");
        }

        var eventType = parts[2];
        var jsonPayload = parts[3];

        // Deserialize using the polymorphic converter
        try
        {
            var evt = JsonSerializer.Deserialize<GameEvent>(jsonPayload, _jsonOptions);
            if (evt == null)
            {
                throw new FormatException($"Deserialized event is null at line {lineNumber}");
            }

            return evt;
        }
        catch (JsonException ex)
        {
            throw new FormatException(
                $"Failed to deserialize JSON at line {lineNumber}: {ex.Message}", ex);
        }
    }

    /// <summary>
    /// Determines if an IOException is due to out of disk space.
    /// </summary>
    private static bool IsOutOfDiskSpace(IOException ex)
    {
        const int ERROR_DISK_FULL = 0x70;
        const int ERROR_HANDLE_DISK_FULL = 0x27;

        var hResult = ex.HResult & 0xFFFF;
        return hResult == ERROR_DISK_FULL || hResult == ERROR_HANDLE_DISK_FULL;
    }
}
