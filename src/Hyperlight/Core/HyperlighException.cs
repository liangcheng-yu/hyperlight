using System;
using Microsoft.Extensions.Logging;
using System.Diagnostics.CodeAnalysis;

namespace Hyperlight.Core
{
    [Serializable]
    public class HyperlightException : Exception
    {
        public HyperlightException(string message) : base(message)
        {
        }

        public HyperlightException(string message, Exception innerException) : base(message, innerException)
        {
        }

        protected HyperlightException(System.Runtime.Serialization.SerializationInfo serializationInfo, System.Runtime.Serialization.StreamingContext streamingContext) : base(serializationInfo, streamingContext)
        {
        }
        public HyperlightException()
        {
        }
        [DoesNotReturn]
        internal static void LogAndThrowException(string message, string correlationId, string source, Exception innerException, LogLevel level = LogLevel.Error, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            var exception = new HyperlightException(GetExceptionMessage(message, correlationId, source), innerException);
            LogAndThrow(level, message, correlationId, source, exception, memberName, source, sourceLineNumber);

        }
        [DoesNotReturn]
        internal static void LogAndThrowException(string message, string correlationId, string source, LogLevel level = LogLevel.Error, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            var exception = new HyperlightException(GetExceptionMessage(message, correlationId, source));
            LogAndThrow(level, message, correlationId, source, exception, memberName, source, sourceLineNumber);
        }
        [DoesNotReturn]
        internal static void LogAndThrowException<T>(string message, string correlationId, string source, LogLevel level = LogLevel.Error, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
            where T : Exception
        {
            var exception = (T)Activator.CreateInstance(typeof(T), GetExceptionMessage(message, correlationId, source))!;
            LogAndThrow(level, message, correlationId, source, exception, memberName, source, sourceLineNumber);
        }
        [DoesNotReturn]
        static void LogAndThrow(LogLevel level, string message, string correlationId, string source, Exception ex, string memberName, string sourceFilePath, int sourceLineNumber)
        {
            ex.Data.Add("CorrelationId", correlationId);
            ex.Data.Add("Source", source);
            HyperlightLogger.Log(level, message, correlationId, source, ex, memberName, sourceFilePath, sourceLineNumber);
            throw ex;
        }
        static string GetExceptionMessage(string message, string correlationId, string source)
        {
            return $"{message} CorrelationId: {correlationId} Source: {source}";
        }

        // Use of [NotNull] Attribute ensures that code analysis does not flag use of variable as possible null reference after this method has been called.

        internal static void ThrowIfNull([NotNull] object? argument, string argName, string correlationId, string source, LogLevel level = LogLevel.Error, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
#pragma warning disable CS8777 // Parameter must have a non-null value when exiting.
        {
            if (argument is null)
            {
                var message = $"{argName} cannot be null";
                var exception = new ArgumentNullException(argName, GetExceptionMessage(message, correlationId, source));
                LogAndThrow(level, message, correlationId, source, exception, memberName, source, sourceLineNumber);
            }
        }

        internal static void ThrowIfNull([NotNull] object? argument, string correlationId, string source, LogLevel level = LogLevel.Error, [System.Runtime.CompilerServices.CallerMemberName] string memberName = "", [System.Runtime.CompilerServices.CallerFilePath] string sourceFilePath = "", [System.Runtime.CompilerServices.CallerLineNumber] int sourceLineNumber = 0)
        {
            if (argument is null)
            {
                var message = "Value cannot be null";
                var exception = new ArgumentNullException(GetExceptionMessage(message, correlationId, source));
                LogAndThrow(level, message, correlationId, source, exception, memberName, source, sourceLineNumber);
            }
        }
#pragma warning restore CS8777 // Parameter must have a non-null value when exiting.
    }
}
