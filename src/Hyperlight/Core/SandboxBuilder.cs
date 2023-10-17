using System;
using System.IO;
using Hyperlight.Core;
using HyperlightDependencies;
using Microsoft.Extensions.Logging;

namespace Hyperlight
{
    public class SandboxBuilder
    {
        private string? guestBinaryPath;
        private SandboxConfiguration? config;
        private SandboxRunOptions? runOptions;
        private Action<ISandboxRegistration>? initFunction;
        private StringWriter? writer;
        private string? correlationId;
        private ILogger? errorMessageLogger;
        private Func<string>? getCorrelationId;
        public SandboxBuilder() { }

        public SandboxBuilder WithGuestBinaryPath(string path)
        {
            this.guestBinaryPath = path;
            return this;
        }

        public SandboxBuilder WithConfig(SandboxConfiguration cfg)
        {
            this.config = cfg;
            return this;
        }

        public SandboxBuilder WithRunOptions(SandboxRunOptions opts)
        {
            this.runOptions = opts;
            return this;
        }

        public SandboxBuilder WithInitFunction(Action<ISandboxRegistration> fn)
        {
            this.initFunction = fn;
            return this;
        }

        public SandboxBuilder WithWriter(StringWriter wr)
        {
            this.writer = wr;
            return this;
        }

        public SandboxBuilder WithCorrelationId(string correlationId)
        {
            this.getCorrelationId = null;
            this.correlationId = correlationId;
            return this;
        }

        public SandboxBuilder WithCorrelationId(Func<string> getCorrelationId)
        {
            this.correlationId = null;
            this.getCorrelationId = getCorrelationId;
            return this;
        }

        public SandboxBuilder WithErrorMessageLogger(ILogger errorMessageLogger)
        {
            this.errorMessageLogger = errorMessageLogger;
            return this;
        }

        public Sandbox Build()
        {
            if (null == this.guestBinaryPath)
            {
                throw new InvalidOperationException("guest binary path is null");
            }
            var runOpts = SandboxRunOptions.None;
            if (null != this.runOptions)
            {
                runOpts = this.runOptions.Value;
            }

            return new Sandbox(
                   this.guestBinaryPath,
                   runOpts,
                   this.initFunction,
                   this.writer,
                   this.correlationId,
                   this.errorMessageLogger,
                   this.config,
                   this.getCorrelationId
               );
        }

    }

}
