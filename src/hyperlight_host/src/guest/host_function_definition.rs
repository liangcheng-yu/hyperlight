use crate::flatbuffers::hyperlight::generated::{
    HostFunctionDefinition as FbHostFunctionDefinition,
    HostFunctionDefinitionArgs as FbHostFunctionDefinitionArgs, ParameterType as FbParameterType,
    ReturnType as FbReturnType,
};
use anyhow::{bail, Result};
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use readonly;

/// The definition of a function exposed from the host to the guest
#[readonly::make]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HostFunctionDefinition {
    /// The function name
    pub function_name: String,
    /// The type of the parameter values for the host function call.
    pub parameter_types: Option<Vec<ParamValueType>>,
    /// The type of the return value from the host function call
    pub return_type: ReturnValueType,
}

/// This is the type of a parameter that can be passed to a host function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamValueType {
    /// Parameter is a signed 32 bit integer.
    Int,
    /// Parameter is a signed 64 bit integer.
    Long,
    /// Parameter is a boolean.
    Boolean,
    /// Parameter is a string.
    String,
    /// Parameter is a vector of bytes.
    VecBytes,
}

/// This is the type of a value that can be returned from a host function.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ReturnValueType {
    #[default]
    /// Return value is a signed 32 bit integer.
    Int,
    /// Return value is a signed 64 bit integer.
    Long,
    /// Return value is a boolean.
    Boolean,
    /// Return value is a string.
    String,
    /// Return value is void.
    Void,
}

impl HostFunctionDefinition {
    /// Create a new `HostFunctionDetails`.
    pub fn new(
        function_name: String,
        parameter_types: Option<Vec<ParamValueType>>,
        return_type: ReturnValueType,
    ) -> Self {
        Self {
            function_name,
            parameter_types,
            return_type,
        }
    }

    /// Create a new `HostFunctionDetails`.
    pub fn convert_to_wipoffset_fbhfdef<'a>(
        &self,
        builder: &mut FlatBufferBuilder<'a>,
    ) -> Result<WIPOffset<FbHostFunctionDefinition<'a>>> {
        let host_function_name = builder.create_string(&self.function_name);
        let return_value_type = self.return_type.clone().into();
        let vec_parameters = match &self.parameter_types {
            Some(vec_pvt) => {
                let num_items = vec_pvt.len();
                let mut parameters: Vec<FbParameterType> = Vec::with_capacity(num_items);
                for pvt in vec_pvt {
                    match pvt {
                        ParamValueType::Int => {
                            parameters.push(FbParameterType::hlint);
                        }
                        ParamValueType::Long => {
                            parameters.push(FbParameterType::hllong);
                        }
                        ParamValueType::Boolean => {
                            parameters.push(FbParameterType::hlbool);
                        }
                        ParamValueType::String => {
                            parameters.push(FbParameterType::hlstring);
                        }
                        ParamValueType::VecBytes => {
                            parameters.push(FbParameterType::hlvecbytes);
                        }
                    };
                }
                Some(builder.create_vector(&parameters))
            }
            None => None,
        };

        let fb_host_function_definition: WIPOffset<FbHostFunctionDefinition> =
            FbHostFunctionDefinition::create(
                builder,
                &FbHostFunctionDefinitionArgs {
                    function_name: Some(host_function_name),
                    return_type: return_value_type,
                    parameters: vec_parameters,
                },
            );

        Ok(fb_host_function_definition)
    }
}

impl From<ReturnValueType> for FbReturnType {
    fn from(value: ReturnValueType) -> Self {
        match value {
            ReturnValueType::Int => FbReturnType::hlint,
            ReturnValueType::Long => FbReturnType::hllong,
            ReturnValueType::String => FbReturnType::hlstring,
            ReturnValueType::Boolean => FbReturnType::hlbool,
            ReturnValueType::Void => FbReturnType::hlvoid,
        }
    }
}

impl TryFrom<FbReturnType> for ReturnValueType {
    type Error = anyhow::Error;
    fn try_from(value: FbReturnType) -> Result<Self> {
        match value {
            FbReturnType::hlint => Ok(ReturnValueType::Int),
            FbReturnType::hllong => Ok(ReturnValueType::Long),
            FbReturnType::hlstring => Ok(ReturnValueType::String),
            FbReturnType::hlbool => Ok(ReturnValueType::Boolean),
            FbReturnType::hlvoid => Ok(ReturnValueType::Void),
            _ => bail!("Unknown return type: {:?}", value),
        }
    }
}

impl TryFrom<FbHostFunctionDefinition<'_>> for HostFunctionDefinition {
    type Error = anyhow::Error;
    fn try_from(value: FbHostFunctionDefinition) -> Result<Self> {
        let function_name = value.function_name().to_string();
        let return_type = value.return_type().try_into()?;
        let parameter_types = match value.parameters() {
            Some(pvt) => {
                let len = pvt.len();
                let mut pv: Vec<ParamValueType> = Vec::with_capacity(len);
                for i in 0..len {
                    let param_type = pvt.get(i);
                    match param_type {
                        FbParameterType::hlint => {
                            pv.push(ParamValueType::Int);
                        }
                        FbParameterType::hllong => {
                            pv.push(ParamValueType::Long);
                        }
                        FbParameterType::hlbool => {
                            pv.push(ParamValueType::Boolean);
                        }
                        FbParameterType::hlstring => {
                            pv.push(ParamValueType::String);
                        }
                        FbParameterType::hlvecbytes => {
                            pv.push(ParamValueType::VecBytes);
                        }
                        _ => {
                            bail!("Unknown parameter type: {:?}", param_type)
                        }
                    };
                }
                Some(pv)
            }
            None => None,
        };

        Ok(Self::new(function_name, parameter_types, return_type))
    }
}

impl TryFrom<&[u8]> for HostFunctionDefinition {
    type Error = anyhow::Error;
    fn try_from(value: &[u8]) -> Result<Self> {
        let fb_host_function_definition = flatbuffers::root::<FbHostFunctionDefinition<'_>>(value)?;
        Self::try_from(fb_host_function_definition)
    }
}

impl TryFrom<&HostFunctionDefinition> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(hfd: &HostFunctionDefinition) -> Result<Vec<u8>> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let host_function_definition = hfd.convert_to_wipoffset_fbhfdef(&mut builder)?;
        builder.finish_size_prefixed(host_function_definition, None);
        Ok(builder.finished_data().to_vec())
    }
}

impl TryFrom<HostFunctionDefinition> for Vec<u8> {
    type Error = anyhow::Error;
    fn try_from(value: HostFunctionDefinition) -> Result<Vec<u8>> {
        (&value).try_into()
    }
}
