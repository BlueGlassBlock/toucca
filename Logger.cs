﻿using System.IO;
using System.Text;

public static class Logger
{
    private static readonly object lockObj = new();
    private static string? logFilePath;

    private static string GetLogFilePath()
    {
        if (logFilePath == null)
        {
            var fileName = $"toucca-{DateTime.Now:yyyy-MM-dd}.log";
            logFilePath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, fileName);
        }
        return logFilePath;
    }


    public static void CleanupOldLogFiles()
    {
        var directory = new DirectoryInfo(AppDomain.CurrentDomain.BaseDirectory);
        var oldFiles = directory.GetFiles("toucca-*.log")
                                 .Where(f => f.CreationTime < DateTime.Now.AddDays(-7))
                                 .ToList();

        foreach (var file in oldFiles)
        {
            try
            {
                file.Delete();
            }
            catch (Exception ex)
            {
                Error("Failed to delete log file", ex);
            }
        }
    }

    private static void LogException(StringBuilder logMessage, Exception ex)
    {

        logMessage.AppendLine();
        logMessage.AppendLine($"Exception: {ex.Message}");
        logMessage.AppendLine($"StackTrace: {ex.StackTrace}");
    }

    public static void Info(string message)
    {
        Log("INFO", message);
    }

    public static void Warn(string message)
    {
        Log("WARN", message);
    }

    public static void Error(string message, Exception? ex = null)
    {
        var logMessage = new StringBuilder(message);
        if (ex != null)
            LogException(logMessage, ex);
        Log("ERROR", logMessage.ToString());
    }

    public static void Fatal(string message, Exception? ex = null)
    {
        var logMessage = new StringBuilder(message);
        if (ex != null)
            LogException(logMessage, ex);
        Log("FATAL", logMessage.ToString());
    }

    private static void Log(string level, string message)
    {
        try
        {
            lock (lockObj)
            {
                using var sw = new StreamWriter(GetLogFilePath(), true, Encoding.UTF8);
                sw.WriteLine($"{DateTime.Now:yyyy-MM-dd HH:mm:ss} [{level}] {message}");
            }
        }
        catch
        {
        }
    }
}
