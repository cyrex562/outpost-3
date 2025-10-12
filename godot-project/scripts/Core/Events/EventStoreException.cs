using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Exception thrown when an event store operation fails.
/// </summary>
/// <remarks>
/// This exception is thrown for various event store failures including:
/// - I/O errors (disk full, permission denied, file corruption)
/// - Serialization/deserialization errors
/// - Invalid event data or offsets
/// </remarks>
public class EventStoreException : Exception
{
    /// <summary>
    /// Creates a new EventStoreException with a message.
    /// </summary>
    /// <param name="message">The error message.</param>
    public EventStoreException(string message) : base(message)
    {
    }

    /// <summary>
    /// Creates a new EventStoreException with a message and inner exception.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <param name="innerException">The underlying exception that caused this error.</param>
    public EventStoreException(string message, Exception innerException) 
        : base(message, innerException)
    {
    }

    /// <summary>
    /// Creates a new EventStoreException for a specific offset.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <param name="offset">The offset where the error occurred.</param>
    public EventStoreException(string message, long offset) 
        : base($"{message} (at offset {offset})")
    {
        Offset = offset;
    }

    /// <summary>
    /// Creates a new EventStoreException for a specific offset with an inner exception.
    /// </summary>
    /// <param name="message">The error message.</param>
    /// <param name="offset">The offset where the error occurred.</param>
    /// <param name="innerException">The underlying exception that caused this error.</param>
    public EventStoreException(string message, long offset, Exception innerException) 
        : base($"{message} (at offset {offset})", innerException)
    {
        Offset = offset;
    }

    /// <summary>
    /// Gets the offset where the error occurred, if applicable.
    /// </summary>
    public long? Offset { get; }
}
