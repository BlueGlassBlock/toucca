using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using System.Windows;

namespace toucca
{
    static class MercuryHelper
    {
        [DllImport("user32.dll", SetLastError = true)]
        static extern IntPtr FindWindow(string? lpClassName, string lpWindowName);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool ClientToScreen(IntPtr hWnd, ref POINT lpPoint);

        [DllImport("user32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);

        [StructLayout(LayoutKind.Sequential)]
        public struct POINT
        {
            public int X;
            public int Y;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct RECT
        {
            public int Left;
            public int Top;
            public int Right;
            public int Bottom;

            public readonly int Width => Right - Left;
            public readonly int Height => Bottom - Top;
        }

        public static bool SetWindowRect(IntPtr hWnd, Rect rect)
        {
            return SetWindowPos(hWnd, IntPtr.Zero, (int)rect.Left, (int)rect.Top, (int)rect.Width, (int)rect.Height, 0x0040);
        }

        public static Rect? TryLocateMecury()
        {
            try
            {
                var hWnd = FindWindow(null, "Mercury  "); // Good job Marvelous Inc.
                if (hWnd != IntPtr.Zero)
                {
                    RECT rect;
                    if (GetWindowRect(hWnd, out rect))
                    {
                        POINT pt = new() { X = 0, Y = 0 };
                        if (ClientToScreen(hWnd, ref pt))
                        {
                            rect.Left = pt.X;
                            rect.Top = pt.Y;
                        }
                        // Calculate the desired size and position based on the other application's window
                        var height = rect.Width - 10;
                        var left = rect.Left; // Center horizontally
                        var top = rect.Top + rect.Height * 0.187;
                        return new Rect(left, top, height, height);
                        // TODO: Properly handle windowed mode
                    }
                }
            }
            catch (Exception ex)
            {
                Logger.Error("Failed to determine Mercury position", ex);
            }

            return null;
        }
    }
}
