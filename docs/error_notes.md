- time advancing off

- panning doesnt work

- asteroid and cometary belts should be rendered as an annulus or wide ring about the orbit

- the colors of bodies should mean something - brown for rocky, green for gas giant, blue for cometary body/ice dwarf, gray for asteroid belts.

- moons should not be rendered in their own orbits. they should be members of a planetary system. dwarf planets should be able to be placed in an orbit, but generally farther out than the gas giants in a system. asteroid belts should be far less common. gas giants, ice giants, and rocky planets should be much more common. 

- draw a line on the galaxy map from the sol system to target system when a probe is flying there.

- exception: E 0:05:04:419   void System.Text.Json.ThrowHelper.ThrowNotSupportedException_DictionaryKeyTypeNotSupported(System.Type, System.Text.Json.Serialization.JsonConverter): Outpost3.Core.Events.EventStoreException: Failed to save snapshot: The type 'System.Ulid' is not a supported dictionary key using converter of type 'Outpost3.Core.Persistence.UlidJsonConverter'. Custom converters can add support for dictionary key serialization by overriding the 'ReadAsPropertyName' and 'WriteAsPropertyName' methods. The unsupported member type is located on type 'Outpost3.Core.Domain.CameraState'. Path: $.GameState.CameraStates. ---> System.NotSupportedException: The type 'System.Ulid' is not a supported dictionary key using converter of type 'Outpost3.Core.Persistence.UlidJsonConverter'. Custom converters can add support for dictionary key serialization by overriding the 'ReadAsPropertyName' and 'WriteAsPropertyName' methods. The unsupported member type is located on type 'Outpost3.Core.Domain.CameraState'. Path: $.GameState.CameraStates. ---> System.NotSupportedException: The type 'System.Ulid' is not a supported dictionary key using converter of type 'Outpost3.Core.Persistence.UlidJsonConverter'. Custom converters can add support for dictionary key serialization by overriding the 'ReadAsPropertyName' and 'WriteAsPropertyName' methods.
  <C++ Error>   Outpost3.Core.Events.EventStoreException
  <C++ Source>  :0 @ void System.Text.Json.ThrowHelper.ThrowNotSupportedException_DictionaryKeyTypeNotSupported(System.Type, System.Text.Json.Serialization.JsonConverter)
  <Stack Trace> :0 @ void System.Text.Json.ThrowHelper.ThrowNotSupportedException_DictionaryKeyTypeNotSupported(System.Type, System.Text.Json.Serialization.JsonConverter)
                :0 @ void System.Text.Json.Serialization.JsonConverter`1.WriteAsPropertyName(System.Text.Json.Utf8JsonWriter, T, System.Text.Json.JsonSerializerOptions)
                :0 @ void System.Text.Json.Serialization.JsonConverter`1.WriteAsPropertyNameCore(System.Text.Json.Utf8JsonWriter, T, System.Text.Json.JsonSerializerOptions, bool)
                :0 @ bool System.Text.Json.Serialization.Converters.DictionaryOfTKeyTValueConverter`3.OnWriteResume(System.Text.Json.Utf8JsonWriter, TCollection, System.Text.Json.JsonSerializerOptions, System.Text.Json.WriteStack&)
                :0 @ bool System.Text.Json.Serialization.JsonDictionaryConverter`3.OnTryWrite(System.Text.Json.Utf8JsonWriter, TDictionary, System.Text.Json.JsonSerializerOptions, System.Text.Json.WriteStack&)
                :0 @ bool System.Text.Json.Serialization.JsonConverter`1.TryWrite(System.Text.Json.Utf8JsonWriter, T&, System.Text.Json.JsonSerializerOptions, System.Text.Json.WriteStack&)
                :0 @ bool System.Text.Json.Serialization.Metadata.JsonPropertyInfo`1.GetMemberAndWriteJson(object, System.Text.Json.WriteStack&, System.Text.Json.Utf8JsonWriter)
                :0 @ bool System.Text.Json.Serialization.Converters.ObjectDefaultConverter`1.OnTryWrite(System.Text.Json.Utf8JsonWriter, T, System.Text.Json.JsonSerializerOptions, System.Text.Json.WriteStack&)
                :0 @ bool System.Text.Json.Serialization.JsonConverter`1.TryWrite(System.Text.Json.Utf8JsonWriter, T&, System.Text.Json.JsonSerializerOptions, System.Text.Json.WriteStack&)
                :0 @ bool System.Text.Json.Serialization.Metadata.JsonPropertyInfo`1.GetMemberAndWriteJson(object, System.Text.Json.WriteStack&, System.Text.Json.Utf8JsonWriter)
                :0 @ bool System.Text.Json.Serialization.Converters.ObjectDefaultConverter`1.OnTryWrite(System.Text.Json.Utf8JsonWriter, T, System.Text.Json.JsonSerializerOptions, System.Text.Json.WriteStack&)
                :0 @ bool System.Text.Json.Serialization.JsonConverter`1.TryWrite(System.Text.Json.Utf8JsonWriter, T&, System.Text.Json.JsonSerializerOptions, System.Text.Json.WriteStack&)
                :0 @ bool System.Text.Json.Serialization.JsonConverter`1.WriteCore(System.Text.Json.Utf8JsonWriter, T&, System.Text.Json.JsonSerializerOptions, System.Text.Json.WriteStack&)
                :0 @ --- End of inner exception stack trace ---()
                :0 @ void System.Text.Json.ThrowHelper.ThrowNotSupportedException(System.Text.Json.WriteStack&, System.Exception)
                :0 @ bool System.Text.Json.Serialization.JsonConverter`1.WriteCore(System.Text.Json.Utf8JsonWriter, T&, System.Text.Json.JsonSerializerOptions, System.Text.Json.WriteStack&)
                :0 @ void System.Text.Json.Serialization.Metadata.JsonTypeInfo`1.Serialize(System.Text.Json.Utf8JsonWriter, T&, object)
                :0 @ string System.Text.Json.JsonSerializer.WriteString<TValue>(TValue&, System.Text.Json.Serialization.Metadata.JsonTypeInfo`1[TValue])
                :0 @ string System.Text.Json.JsonSerializer.Serialize<TValue>(TValue, System.Text.Json.JsonSerializerOptions)
                JsonSnapshotStore.cs:54 @ void Outpost3.Core.Persistence.JsonSnapshotStore.SaveSnapshot(Outpost3.Core.Domain.GameState, long, Outpost3.Core.Domain.SaveMetadata)
                :0 @ --- End of inner exception stack trace ---()
                JsonSnapshotStore.cs:63 @ void Outpost3.Core.Persistence.JsonSnapshotStore.SaveSnapshot(Outpost3.Core.Domain.GameState, long, Outpost3.Core.Domain.SaveMetadata)
                SaveLoadService.cs:38 @ void Outpost3.Core.Services.SaveLoadService.SaveGame(string, string)
                SaveLoadService.cs:97 @ void Outpost3.Core.Services.SaveLoadService.AutoSave()
                GameServices.cs:145 @ void Outpost3.GameServices.OnAutoSave()
                Callable.generics.cs:39 @ void Godot.Callable.<From>g__Trampoline|1_0(object, Godot.NativeInterop.NativeVariantPtrArgs, Godot.NativeInterop.godot_variant&)
                DelegateUtils.cs:86 @ void Godot.DelegateUtils.InvokeWithVariantArgs(nint, System.Void*, Godot.NativeInterop.godot_variant**, int, Godot.NativeInterop.godot_variant*)
