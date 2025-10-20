// Package melody is a templating and parsing library for Cohere models
package melody

import (
	"fmt"
	"strings"

	"github.com/cohere-ai/melody/lib/orderedjson"
)

func reverse[A comparable, B comparable](m map[A]B) map[B]A {
	reversed := make(map[B]A)
	for k, v := range m {
		reversed[v] = k
	}
	return reversed
}

type Encoding interface {
	Render(Messages []Message) (string, error)
	ProcessToken(int64) error
}

type Document = string

type Tool struct {
	Name        string
	Description string
	Parameters  orderedjson.Object
}

type Message struct {
	Role       Role
	Content    []Content
	ToolCalls  []ToolCall
	ToolCallID string

	AdditionalFields map[string]any
}

type Content struct {
	Type     ContentType
	Text     string
	Thinking string
	Image    *Image
}

type Image struct {
	TemplatePlaceholder string
}

type ToolCall struct {
	ID         string
	Name       string
	Parameters orderedjson.Object
}

type Role struct{ e int }

var UnknownRole = Role{e: 0}
var SystemRole = Role{e: 1}
var UserRole = Role{e: 2}
var ChatbotRole = Role{e: 3}
var ToolRole = Role{e: 4}
var roleToString = map[Role]string{
	SystemRole:  "system",
	UserRole:    "user",
	ChatbotRole: "chatbot",
	ToolRole:    "tool",
}
var stringToRole = reverse(roleToString)

func RoleFromString(s string) (Role, error) {
	if r, ok := stringToRole[strings.ToLower(s)]; ok {
		return r, nil
	}
	return UnknownRole, fmt.Errorf("unknown role: %s", s)
}
func (r Role) String() string {
	return strings.ToUpper(roleToString[r])
}

type ContentType struct{ e int }

var UnknownContentType = ContentType{e: 0}
var TextContentType = ContentType{e: 1}
var ThinkingContentType = ContentType{e: 2}
var ImageContentType = ContentType{e: 3}
var contentTypeToString = map[ContentType]string{
	TextContentType:     "text",
	ThinkingContentType: "thinking",
	ImageContentType:    "image_url",
}
var stringToContentType = reverse(contentTypeToString)

func ContentTypeFromString(s string) (ContentType, error) {
	if ct, ok := stringToContentType[strings.ToLower(s)]; ok {
		return ct, nil
	}
	return UnknownContentType, fmt.Errorf("unknown content type: %s", s)
}
func (c ContentType) String() string {
	return strings.ToUpper(contentTypeToString[c])
}

type CitationQuality struct{ e int }

var UnknownCitationQuality = CitationQuality{e: 0}
var OffCitationQuality = CitationQuality{e: 1}
var OnCitationQuality = CitationQuality{e: 2}
var citationQualityToString = map[CitationQuality]string{
	OffCitationQuality: "off",
	OnCitationQuality:  "on",
}
var stringToCitationQuality = reverse(citationQualityToString)

func CitationQualityFromString(s string) (CitationQuality, error) {
	if cq, ok := stringToCitationQuality[strings.ToLower(s)]; ok {
		return cq, nil
	}
	return UnknownCitationQuality, fmt.Errorf("unknown citation quality: %s", s)
}
func (c CitationQuality) String() string {
	return strings.ToUpper(citationQualityToString[c])
}

type Grounding struct{ e int }

var UnknownGrounding = Grounding{e: 0}
var EnabledGrounding = Grounding{e: 1}
var DisabledGrounding = Grounding{e: 2}
var groundingToString = map[Grounding]string{
	EnabledGrounding:  "enabled",
	DisabledGrounding: "disabled",
}
var stringToGrounding = reverse(groundingToString)

func GroundingFromString(s string) (Grounding, error) {
	if cq, ok := stringToGrounding[strings.ToLower(s)]; ok {
		return cq, nil
	}
	return UnknownGrounding, fmt.Errorf("unknown grounding value: %s", s)
}
func (c Grounding) String() string {
	return strings.ToUpper(groundingToString[c])
}

type SafetyMode struct{ e int }

var UnknownSafetyMode = SafetyMode{e: 0}
var NoneSafetyMode = SafetyMode{e: 1}
var StrictSafetyMode = SafetyMode{e: 2}
var ContextualSafetyMode = SafetyMode{e: 3}
var safetyModeToString = map[SafetyMode]string{
	NoneSafetyMode:       "none",
	StrictSafetyMode:     "strict",
	ContextualSafetyMode: "contextual",
}
var stringToSafetyMode = reverse(safetyModeToString)

func SafetyModeFromString(s string) (SafetyMode, error) {
	if sm, ok := stringToSafetyMode[strings.ToLower(s)]; ok {
		return sm, nil
	}
	return UnknownSafetyMode, fmt.Errorf("unknown safety mode: %s", s)
}
func (s SafetyMode) String() string {
	return strings.ToUpper(safetyModeToString[s])
}

type ReasoningType struct{ e int }

var UnknownReasoningType = ReasoningType{e: 0}
var EnabledReasoningType = ReasoningType{e: 1}
var DisabledReasoningType = ReasoningType{e: 2}
var reasoningTypeToString = map[ReasoningType]string{
	EnabledReasoningType:  "enabled",
	DisabledReasoningType: "disabled",
}
var stringToReasoningType = reverse(reasoningTypeToString)

func ReasoningTypeFromString(s string) (ReasoningType, error) {
	if rt, ok := stringToReasoningType[strings.ToLower(s)]; ok {
		return rt, nil
	}
	return UnknownReasoningType, fmt.Errorf("unknown reasoning type: %s", s)
}
func (r ReasoningType) String() string {
	return strings.ToUpper(reasoningTypeToString[r])
}
