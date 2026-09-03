module Core.Errors

type Error
	case ProtocolError(cause) end
	case ArgumentError(cause) end
	case TypeError(cause) end
	case MemberError(cause) end
	case IndexError(cause) end
	case NameError(message) end
	case KeyError(message) end
end

type ProtocolError
	case NotIterable(message) end
	case NotAccessible(message) end
	case NotAppendable(message) end
	case NotOrderable(message) end
	case NotInspectable(message) end
	case NotDisplayable(message) end
	case NotHashable(message) end
	case NotImplemented(message) end
end

type ArgumentError
	case Missing(message) end
	case TooMany(message) end
	case Unknown(message) end
	case Duplicate(message) end
end

type TypeError
	case IndexNonNum(message) end
	case ArithNonNum(message) end
	case ConcatNonStr(message) end
	case MixNonDigest(message) end
	case NotCallable(message) end
	case NotConstructible(message) end
	case NotInvocable(message) end
	case CaseNonType(message) end
end

type MemberError
	case Missing(message) end
	case ReadOnly(message) end
end

type IndexError
	case OutOfRange(message) end
	case NonIntegral(message) end
end
