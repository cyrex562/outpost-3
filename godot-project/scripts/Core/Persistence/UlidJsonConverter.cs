using System;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Outpost3.Core.Persistence;

/// <summary>
/// JSON converter for Ulid type.
/// Enables serialization and deserialization of Ulid values to/from JSON strings.
/// </summary>
public class UlidJsonConverter : JsonConverter<Ulid>
{
    public override Ulid Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        var str = reader.GetString();
        return string.IsNullOrEmpty(str) ? Ulid.Empty : Ulid.Parse(str);
    }
        
    public override void Write(Utf8JsonWriter writer, Ulid value, JsonSerializerOptions options)
    {
        writer.WriteStringValue(value.ToString());
    }
}
