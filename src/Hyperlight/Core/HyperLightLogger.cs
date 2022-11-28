using System;
using System.Reflection;
using Microsoft.Extensions.Logging;

namespace Hyperlight.Core
{
    public static partial class HyperlightLogger
    {
        static readonly string version = Assembly.GetExecutingAssembly().GetName().Version?.ToString() ?? "unknown";

        static ILogger logger = GetDefaultLogger();

        static readonly Func<string?, string?, LogLevel, bool> filter = (_, _, level) => level > LogLevel.Information;

        /// <summary>
        /// Set the ILogger to be used by Hyperlight .Defaults to the debug logger.
        /// </summary>

        public static void SetLogger(ILogger? logger) => HyperlightLogger.logger = logger ?? GetDefaultLogger();

        static ILogger GetDefaultLogger()
        {
            using var loggerFactory = LoggerFactory.Create(builder =>
            {
                builder
                    .AddFilter(filter)
                    .AddDebug();
            });

            return loggerFactory.CreateLogger<Hyperlight.Sandbox>();
        }

        [LoggerMessage(EventId = 1, Message = "ErrorMessage: {message} CorrelationId: {correlationId} Source: {source} HyperLightVersion: {version} Caller: {memberName} SourceFile: {sourceFilePath} Line: {sourceLineNumber:n0}")]
        static partial void log(LogLevel level, ILogger logger, string message, string correlationId, string source, string version, string memberName, string sourceFilePath, int sourceLineNumber, Exception? ex = null);

        [LoggerMessage(EventId = 2, Message = "ErrorMessage: {message} CorrelationId: {correlationId} Source: {source} HyperLightVersion: {version} Caller: {memberName} SourceFile: {sourceFilePath} Line: {sourceLineNumber:n0}", Level = LogLevel.Error)]
        static partial void logError(ILogger logger, string message, string correlationId, string source, string version, string memberName, string sourceFilePath, int sourceLineNumber, Exception? ex = null);

        [LoggerMessage(EventId = 3, Message = "ErrorMessage: {message} CorrelationId:{correlationId} Source: {source} HyperLightVersion: {version} Caller: {memberName} SourceFile: {sourceFilePath} Line: {sourceLineNumber:n0}", Level = LogLevel.Information)]
        static partial void logInformation(ILogger logger, string message, string correlationId, string source, string version, string memberName, string sourceFilePath, int sourceLineNumber, Exception? ex = null);

        [LoggerMessage(EventId = 4, Message = "ErrorMessage: {message} CorrelationId:{correlationId} Source: {source} HyperLightVersion: {version} Caller: {memberName} SourceFile: {sourceFilePath} Line: {sourceLineNumber:n0}", Level = LogLevel.Trace)]
        static partial void logTrace(ILogger logger, string message, string correlationId, string source, string version, string memberName, string sourceFilePath, int sourceLineNumber, Exception? ex = null);

        [LoggerMessage(EventId = 5, Message = "ErrorMessage: {message} CorrelationId:{correlationId} Source: {source} HyperLightVersion: {version} Caller: {memberName} SourceFile: {sourceFilePath} Line: {sourceLineNumber:n0}", Level = LogLevel.Debug)]
        static partial void logDebug(ILogger logger, string message, string correlationId, string source, string version, string memberName, string sourceFilePath, int sourceLineNumber, Exception? ex = null);

        [LoggerMessage(EventId = 6, Message = "ErrorMessage: {message} CorrelationId:{correlationId} Source: {source} HyperLightVersion: {version} Caller: {memberName} SourceFile: {sourceFilePath} Line: {sourceLineNumber:n0}", Level = LogLevel.Warning)]
        static partial void logWarning(ILogger logger, string message, string correlationId, string source, string version, string memberName, string sourceFilePath, int sourceLineNumber, Exception? ex = null);

        [LoggerMessage(EventId = 7, Message = "ErrorMessage: {message} CorrelationId:{correlationId} Source: {source} HyperLightVersion: {version} Caller: {memberName} SourceFile: {sourceFilePath} Line: {sourceLineNumber:n0}", Level = LogLevel.Critical)]
        static partial void logCritical(ILogger logger, string message, string correlationId, string source, string version, string memberName, string sourceFilePath, int sourceLineNumber, Exception? ex = null);

        public static void LogError(string message, string correlationId, string source, Exception? ex = null, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            logError(logger, message, correlationId, source, version, memberName, sourceFilePath, sourceLineNumber, ex);
        }

        public static void LogInformation(string message, string correlationId, string source, Exception? ex = null, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            logInformation(logger, message, correlationId, source, version, memberName, sourceFilePath, sourceLineNumber, ex);
        }

        public static void LogTrace(string message, string correlationId, string source, Exception? ex = null, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            logTrace(logger, message, correlationId, source, version, memberName, sourceFilePath, sourceLineNumber, ex);
        }

        public static void LogDebug(string message, string correlationId, string source, Exception? ex = null, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            logDebug(logger, message, correlationId, source, version, memberName, sourceFilePath, sourceLineNumber, ex);
        }

        public static void LogWarning(string message, string correlationId, string source, Exception? ex = null, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            logWarning(logger, message, correlationId, source, version, memberName, sourceFilePath, sourceLineNumber, ex);
        }

        public static void LogCritical(string message, string correlationId, string source, Exception? ex = null, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            logCritical(logger, message, correlationId, source, version, memberName, sourceFilePath, sourceLineNumber, ex);
        }

        public static void Log(LogLevel level, string message, string correlationId, string source, Exception? ex = null, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            log(level, logger, message, correlationId, source, version, memberName, sourceFilePath, sourceLineNumber, ex);
        }
    }
}
