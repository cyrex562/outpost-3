using System;
using System.Runtime.InteropServices;
using System.Text;
using Godot;

public static class RustBridge
{
    private const string LibName = "outpost_3_core";

    [DllImport(LibName)]
    private static extern IntPtr game_state_new();

    [DllImport(LibName)]
    private static extern void game_state_free(IntPtr handle);

    [DllImport(LibName)]
    private static extern IntPtr game_state_apply_command(IntPtr handle, string command);

    [DllImport(LibName)]
    private static extern IntPtr game_state_to_json(IntPtr handle);

    [DllImport(LibName)]
    private static extern void game_string_free(IntPtr str);

    public static IntPtr CreateGameState()
    {
        return game_state_new();
    }

    public static void FreeGameState(IntPtr handle)
    {
        game_state_free(handle);
    }

    public static string ApplyCommand(IntPtr handle, string commandJson)
    {
        IntPtr resultPtr = game_state_apply_command(handle, commandJson);
        string result = Marshal.PtrToStringUTF8(resultPtr);
        game_string_free(resultPtr);
        return result;
    }

    public static string GetStateJson(IntPtr handle)
    {
        IntPtr jsonPtr = game_state_to_json(handle);
        string json = Marshal.PtrToStringUTF8(jsonPtr);
        game_string_free(jsonPtr);
        return json;
    }

}