using System;
using System.Collections.Generic;
using System.Collections.Concurrent;
using System.Reflection;
using System.Reflection.Emit;
using Hyperlight.Core;
using HyperlightDependencies;

namespace Hyperlight
{
    internal struct HostMethodInfo
    {
        public object? target;
        public MethodInfo methodInfo;
    }
    abstract class GuestInterfaceGlue
    {

        // Currently we will support int, Int64, bool, byte[] and string for parameters and return types of long and int 
        // for the methods between guest and host
        static readonly HashSet<Type> supportedParameterAndReturnTypes = new() { typeof(int), typeof(long), typeof(ulong), typeof(bool), typeof(string), typeof(byte[]), typeof(IntPtr), typeof(UInt32) };
        static readonly ConcurrentDictionary<string, Lazy<DynamicMethod>> dynamicMethods = new();
        public Dictionary<string, HostMethodInfo> MapHostFunctionNamesToMethodInfo = new();

        public void ExposeAndBindMembers(object guestObjectOrType)
        {

            // The type is either passed to us if we bind to static members, or
            // an instance of an object is passed to us if we bind to its instance members
            // In either case, we need to know what 'Type' we are working with

            // See if we were passed a Type or some other object and then get it's type
            var type = (guestObjectOrType as Type) ?? guestObjectOrType.GetType();

            // Set the target to NULL if we are given a type, or the object if we are given an object
            var target = (guestObjectOrType is Type) ? null : guestObjectOrType;


            // If the type has no ExposeToGuestAttribute, or if the Expose property is set to true then enumerate all the members on the type to expose to the guest.
            // the attribute may also be set on members so members are still enumerated.

            var exposeToGuestAttribute = type.GetCustomAttribute<ExposeToGuestAttribute>();
            var exposeMembers = true;
            if (exposeToGuestAttribute != null && !exposeToGuestAttribute.Expose)
            {
                exposeMembers = false;
            }

            // Generate dymanic methods for each delegate.

            var bindingFlags = BindingFlags.Public;

            // Only include delegates if we have a target.
            // Static delegates are excluded as they need to be bound to a guest function which is by definition instance scoped.
            // This would not work if we create multiple instances of the sandbox exposing members from the same type.

            if (target != null)
            {
                bindingFlags |= BindingFlags.Instance;

                // Look for each delegate field and provide an implementation
                foreach (var fieldInfo in type.GetFields(bindingFlags))
                {
                    // See if this field is a delegate
                    if (typeof(Delegate).IsAssignableFrom(fieldInfo.FieldType))
                    {
                        if (!ShouldExposeMember(fieldInfo.GetCustomAttribute<ExposeToGuestAttribute>(), exposeMembers))
                        {
                            // TODO implement logging rather than using console.write                           
                            Console.WriteLine($"Skipping delegate {fieldInfo.Name} as it is excluded using ExposeToGuestAttribute.");
                            continue;
                        }

                        if (fieldInfo.GetValue(target) != null)
                        {
                            // TODO implement logging rather than using console.write
                            Console.WriteLine($"Skipping delegate {fieldInfo.Name} as it has an implementation. Use the ExposeToGuestAttribute to explictly exclude this delegate");
                            continue;
                        }


                        CreateDymanicMethod(fieldInfo, target);
                    }
                }
            }

            // check to see if there are any static delegates without an implementation, it is not supported to bind Guest methods to static delegates. 

            bindingFlags = BindingFlags.DeclaredOnly | BindingFlags.Public | BindingFlags.Static;

            foreach (var fieldInfo in type.GetFields(bindingFlags))
            {
                // See if this field is a delegate
                if (typeof(Delegate).IsAssignableFrom(fieldInfo.FieldType))
                {
                    if (ShouldExposeMember(fieldInfo.GetCustomAttribute<ExposeToGuestAttribute>(), exposeMembers) && fieldInfo.GetValue(null) == null)
                    {
                        // TODO implement logging rather throwing exception                       
                        throw new HyperlightException($"Skipping delegate ${fieldInfo.Name} as it is static. Use ExposeToGuestAttribute[false] to exclude this member");
                    }
                }
            }

            // Expose Host methods to guest.

            // Only include instance methods if we have a target.

            if (target != null)
            {
                bindingFlags |= BindingFlags.Instance;
            }

            // Get method info for each host helper method
            foreach (var methodInfo in type.GetMethods(bindingFlags))
            {
                ExposeHostMethod(methodInfo, target, exposeMembers);
            }
        }

        private void ExposeHostMethod(MethodInfo methodInfo, object? target = null, bool exposeMembers = true)
        {
            // Validate that we support parameter list and return type
            if (ShouldExposeMember(methodInfo.GetCustomAttribute<ExposeToGuestAttribute>(), exposeMembers))
            {
                ValidateMethodSupported(methodInfo);

                // TODO: Add virtual to attribute or use full name to  allow method name clashes.

                if (!MapHostFunctionNamesToMethodInfo.TryAdd(methodInfo.Name, new HostMethodInfo { methodInfo = methodInfo, target = target }))
                {
                    // TODO Log a warning here
                    MapHostFunctionNamesToMethodInfo[methodInfo.Name] = new HostMethodInfo { methodInfo = methodInfo, target = target };
                }
            }
        }
        private void CreateDymanicMethod(FieldInfo fieldInfo, object target)
        {
            var dynamicMethod = CreateOrAddDynamicMethod(fieldInfo, target);
            fieldInfo.SetValue(target, dynamicMethod.Value.CreateDelegate(fieldInfo.FieldType, this));
        }

        private Lazy<DynamicMethod> CreateOrAddDynamicMethod(FieldInfo fieldInfo, object target)
        {

            return dynamicMethods.GetOrAdd(fieldInfo.Name, _ => new Lazy<DynamicMethod>(() =>
            {
                // Get the Invoke method
                var invokeMethod = fieldInfo.FieldType.GetMethod("Invoke");

                // Validate that we support parameter list and return type
                ValidateMethodSupported(invokeMethod);

                // Build delegate implementation that calls DispatchToGuest the right number of parameters.  This internally calls the abstract DispatchCallsFromHost
                // where the real work can be done to call into the guest.  We don't directly try to generate a call to DispatchCallsFromHost because
                // it is easier NOT to create an object[] in IL

                // Get delegate parameter list
                var parameters = invokeMethod!.GetParameters();

                // Our delegate will be bound to an instance of GuestInterfaceGlue so the first parameter will be typeof(GuestInterfaceGlue)
                // After that, the parameters will match the delegate we are trying to implement
                var delegateParameters = new List<Type>() { this.GetType() };
                foreach (var parameter in parameters)
                {
                    delegateParameters.Add(parameter.ParameterType);
                }

                // Create dynamic method
                var dynamicMethod = new DynamicMethod($"{fieldInfo.Name}", invokeMethod.ReturnType, delegateParameters.ToArray(), this.GetType().Module, true);

                // We are going to create a delegate that looks like this:
                //
                //   public object GuestFunction(int o1, int o2, bool o3, int o4, bool o5, int o6, bool o7, Byte[] o8, int o9, int o10, int o11) 
                //   {   
                //      bool shouldReset=EnterDynamicMethod();
                //      try
                //      {
                //          if (shouldReset)
                //          {
                //              ResetState();
                //          }
                //          return DispatchCallFromHost("GuestFunction", new object[] {o1,o2,o3,o4,o5,o6,o7,o8,o9,o10,o11});
                //      }
                //      finally
                //      {
                //          ExitDynamicMethod(shouldReset);
                //      }
                //   }
                //
                // We basically want to turn an early bound call that the host defined into a call to DispatchCallFromHost(string functionName, object[] args)
                // where the early bound parameters are passed as an object[], boxing if necessary
                // the calls to EnterDyamicMethod and ExitDynamicMethod perform the checks to see if this Sandbox has been used already,
                // if it has, then it check to see if it can it be recycled and it can performs the recycle. If the Sandbox has been used and it cannot be recycled
                // EnterDynamicMethod will throw an exception.

                // Get an ILGenerator and emit a body for the dynamic method
                var il = dynamicMethod.GetILGenerator(256);

                // Create a local variable for the result of EnterDynamicMethod and set to false;

                il.DeclareLocal(typeof(bool));
                il.Emit(OpCodes.Ldc_I4_0);
                il.Emit(OpCodes.Stloc_0);

                // Create a local variable for the result of DispatachCallFromHost and set to null;

                il.DeclareLocal(typeof(object));
                il.Emit(OpCodes.Ldnull);
                il.Emit(OpCodes.Stloc_1);

                il.Emit(OpCodes.Ldarg_0);
                var enterDynamicMethod = typeof(GuestInterfaceGlue).GetMethod("EnterDynamicMethod", BindingFlags.NonPublic | BindingFlags.Instance);
                ArgumentNullException.ThrowIfNull(enterDynamicMethod, nameof(enterDynamicMethod));
                il.Emit(OpCodes.Callvirt, enterDynamicMethod);
                il.Emit(OpCodes.Stloc_0);

                var exceptionBlock = il.BeginExceptionBlock();

                // Check the return value from EnterDynamic method if its true we need to call ResetState to check if the Sandbox state allows the call to proceed
                // and if necessary reset the state of the sandbox.

                var noreset = il.DefineLabel();
                il.Emit(OpCodes.Ldloc_0);
                il.Emit(OpCodes.Brfalse, noreset);
                il.Emit(OpCodes.Ldarg_0);
                var resetState = typeof(GuestInterfaceGlue).GetMethod("ResetState", BindingFlags.NonPublic | BindingFlags.Instance);
                ArgumentNullException.ThrowIfNull(resetState, nameof(resetState));
                il.Emit(OpCodes.Callvirt, resetState);
                il.MarkLabel(noreset);

                // First parameter to DispatchCallFromHost the GuestInterfaceGlue 'this' pointer that will be passed to the delegate
                il.Emit(OpCodes.Ldarg_0);

                // Second parameter to DispatchCallFromHost is the name of the function being called
                il.Emit(OpCodes.Ldstr, fieldInfo.Name);

                // Local helper function that does an Emit of Ldc_I4_0/Ldc_I4_1/Ldc_I4_2/.../Ldc_I4_3, or "Ldarg_s "i if 'i' is greater than 8
                void EmitLoadInt(byte i)
                {
                    switch (i)
                    {
                        case 0:
                            il.Emit(OpCodes.Ldc_I4_0);
                            break;
                        case 1:
                            il.Emit(OpCodes.Ldc_I4_1);
                            break;
                        case 2:
                            il.Emit(OpCodes.Ldc_I4_2);
                            break;
                        case 3:
                            il.Emit(OpCodes.Ldc_I4_3);
                            break;
                        case 4:
                            il.Emit(OpCodes.Ldc_I4_4);
                            break;
                        case 5:
                            il.Emit(OpCodes.Ldc_I4_5);
                            break;
                        case 6:
                            il.Emit(OpCodes.Ldc_I4_6);
                            break;
                        case 7:
                            il.Emit(OpCodes.Ldc_I4_7);
                            break;
                        case 8:
                            il.Emit(OpCodes.Ldc_I4_8);
                            break;
                        default:
                            il.Emit(OpCodes.Ldc_I4_S, i);
                            break;
                    }
                }

                // Create object[] with a length equal to the number of parameters in the delegate
                if (parameters.Length > 255)
                {
                    throw new HyperlightException("Hyperlight does not support calling a function with more than 255 parameters");
                }

                EmitLoadInt((byte)parameters.Length);
                il.Emit(OpCodes.Newarr, typeof(object));

                // Put all the parameters into the new object[], boxing as necessary
                for (var i = 0; i < parameters.Length; i++)
                {
                    // Load the array we created
                    il.Emit(OpCodes.Dup);

                    // Load the index where we want to put this parameter
                    EmitLoadInt((byte)i);

                    // Load the passed parameter
                    // For the 2nd to 4th parameter, use Ldarg_1, Ldarg_2, Ldarg_3.  Then use Ldarg_S for all others.
                    // Note - the "first" parameter passed is the 'this pointer'
                    switch (i)
                    {
                        case 0:
                            il.Emit(OpCodes.Ldarg_1);
                            break;
                        case 1:
                            il.Emit(OpCodes.Ldarg_2);
                            break;
                        case 2:
                            il.Emit(OpCodes.Ldarg_3);
                            break;
                        default:
                            il.Emit(OpCodes.Ldarg_S, i + 1);
                            break;
                    }

                    // Box if necessary
                    if (parameters[i].ParameterType.IsValueType)
                    {
                        il.Emit(OpCodes.Box, parameters[i].ParameterType);
                    }

                    // Store the object in the array
                    il.Emit(OpCodes.Stelem_Ref);
                }

                // Emit call to DispatchCallFromHost
                var dispatchCallFromHost = typeof(GuestInterfaceGlue).GetMethod("DispatchCallFromHost", BindingFlags.NonPublic | BindingFlags.Instance);
                ArgumentNullException.ThrowIfNull(dispatchCallFromHost, nameof(dispatchCallFromHost));
                il.EmitCall(OpCodes.Callvirt, dispatchCallFromHost, null);

                // See if we need to unbox
                if (invokeMethod.ReturnType.IsValueType)
                {
                    il.Emit(OpCodes.Unbox_Any, invokeMethod.ReturnType);
                }

                //store return value in the first variable defined above

                il.Emit(OpCodes.Stloc_1);
                il.Emit(OpCodes.Leave, exceptionBlock);

                // End Try 

                il.BeginFinallyBlock();

                il.Emit(OpCodes.Ldarg_0);
                // The argument is the return value from EnterDynamicMethod which is stored in the second variable declared above
                il.Emit(OpCodes.Ldloc_0);
                var exitDynamicMethod = typeof(GuestInterfaceGlue).GetMethod("ExitDynamicMethod", BindingFlags.NonPublic | BindingFlags.Instance);
                ArgumentNullException.ThrowIfNull(exitDynamicMethod, nameof(exitDynamicMethod));
                il.Emit(OpCodes.Callvirt, exitDynamicMethod);

                il.EndExceptionBlock();

                //End Finally

                // push the return value from first variable and return

                il.Emit(OpCodes.Ldloc_1);
                il.Emit(OpCodes.Ret);

                // Get the delegate and assign to the field

                return dynamicMethod;
            }));

        }

        public void BindGuestFunctionToDelegate(string memberName, object instance)
        {
            var fieldInfo = instance.GetType().GetField(memberName, BindingFlags.IgnoreCase | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
            if (fieldInfo == null || fieldInfo.FieldType.BaseType != typeof(MulticastDelegate))
            {
                throw new ArgumentException($"{memberName} is not a delegate.");
            }

            if (fieldInfo.GetValue(instance) != null)
            {
                throw new ArgumentException($"{memberName} already has a value.");
            }

            CreateDymanicMethod(fieldInfo, instance);
        }

        public void ExposeHostMethod(string methodName, object instance)
        {
            var methodInfo = instance.GetType().GetMethod(methodName, BindingFlags.IgnoreCase | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
            if (methodInfo == null)
            {
                throw new ArgumentException($"{methodName} not found.");
            }
            ExposeHostMethod(methodInfo, instance);
        }

        public void ExposeHostMethod(string methodName, Type type)
        {
            var methodInfo = type.GetMethod(methodName, BindingFlags.IgnoreCase | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static);
            if (methodInfo == null)
            {
                throw new ArgumentException($"{methodName} not found.");
            }
            ExposeHostMethod(methodInfo);
        }

        private static bool ShouldExposeMember(ExposeToGuestAttribute? exposeToGuestAttribute, bool exposeMembers) => exposeMembers ? exposeToGuestAttribute == null || exposeToGuestAttribute.Expose : exposeToGuestAttribute != null && exposeToGuestAttribute.Expose;

        public object? DispatchCallFromGuest(string functionName, object[] args)
        {
            if (!MapHostFunctionNamesToMethodInfo.ContainsKey(functionName))
            {
                throw new HyperlightException($"Host does not have helper function name {functionName}");
            }

            var hostMethodInfo = MapHostFunctionNamesToMethodInfo[functionName];

            // Validate paramters
            var parameters = hostMethodInfo.methodInfo.GetParameters();
            if (parameters.Length != args.Length)
            {
                throw new HyperlightException($"Passed wrong number of arguments - Expected {parameters.Length} Passed {args.Length}");
            }

            for (var i = 0; i < parameters.Length; i++)
            {
                // Check to make sure we are passed the expected argument types.
                // NOTE - This check is MORE restrictive than Invoke().  For example, we could pass an 'int' to an 'Int64' but the latter isn't 'assignable from' the former
                // We could make this more relaxed in the future
                if (!parameters[i].ParameterType.IsAssignableFrom(args[i].GetType()))
                {
                    throw new HyperlightException($"Passed argument that is not assignable to the expected type - Expected {parameters[i].ParameterType} Passed {args[i].GetType()}");
                }
            }

            // Call the host helper method
            return hostMethodInfo.methodInfo.Invoke(hostMethodInfo.target, args);
        }

        protected abstract object DispatchCallFromHost(string functionName, object[] args);

        protected abstract bool EnterDynamicMethod();

        protected abstract void ExitDynamicMethod(bool shouldRelease);

        protected abstract void ResetState();

        // Validate that we support the parameter count, parameter types, and return value
        // Throws exception if not supported.  Note that void is supported as a return type
        static void ValidateMethodSupported(MethodInfo? methodInfo)
        {
            ArgumentNullException.ThrowIfNull(methodInfo, nameof(methodInfo));
            var parameters = methodInfo.GetParameters();

            // Currently we only support up to 10 parameters
            if (parameters.Length > Constants.MAX_NUMBER_OF_GUEST_FUNCTION_PARAMETERS)
            {
                throw new HyperlightException($"Method {methodInfo.Name} has too many parameters. Maximum of {Constants.MAX_NUMBER_OF_GUEST_FUNCTION_PARAMETERS} allowed");
            }

            // Check if each parameter is a supported type
            foreach (var parameter in parameters)
            {
                if (!supportedParameterAndReturnTypes.Contains(parameter.ParameterType))
                {
                    throw new HyperlightException($"Unsupported Paramter Type {parameter.ParameterType} on parameter {parameter.Name} method {methodInfo.Name}");
                }
            }

            // Check if return value is a supported type of 'void'
            if (!(methodInfo.ReturnType == typeof(void)) && !supportedParameterAndReturnTypes.Contains(methodInfo.ReturnType))
            {
                throw new HyperlightException($"Unsupported Return Type {methodInfo.ReturnType} on method {methodInfo.Name}");
            }
        }
    }
}
