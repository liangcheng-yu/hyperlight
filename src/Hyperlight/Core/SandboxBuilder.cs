using System;
using System.IO;
using Hyperlight.Core;
using HyperlightDependencies;

namespace Hyperlight
{
    public class SandboxBuilder
    {
        private string? guestBinaryPath;
        private SandboxMemoryConfiguration? config;
        private SandboxRunOptions? runOptions;
        private Action<ISandboxRegistration>? initFunction;
        private StringWriter? writer;
        public SandboxBuilder() { }

        public SandboxBuilder WithGuestBinaryPath(string path)
        {
            this.guestBinaryPath = path;
            return this;
        }

        public SandboxBuilder WithConfig(SandboxMemoryConfiguration cfg)
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

        public Sandbox Build()
        {
            if (null == this.config)
            {
                throw new InvalidOperationException("SandboxMemoryConfig is null");
            }
            else if (null == this.guestBinaryPath)
            {
                throw new InvalidOperationException("guest binary path is null");
            }
            var runOpts = SandboxRunOptions.None;
            if (null != this.runOptions)
            {
                runOpts = this.runOptions.Value;
            }

            return new Sandbox(
                this.config,
                this.guestBinaryPath,
                runOpts,
                this.initFunction,
                this.writer
            );
        }

    }

}
