package parsing

import (
	"strconv"
	"strings"
	"unicode/utf8"

	"go.uber.org/zap"
)

const (
	startFirstCit = "<co: "
	startLastCit  = "</co: "
	endOfCit      = ">"

	startFirstCitCmd3 = "<co"
)

// Input 1. curIndex is the index of the string without citations so far 2. is the string to process e.g. " <co: 1"
// Output 1. output with text and citations 2. amount to remove from the buffer including citations
func (f *filter) ParseCitations(str string, mode filterMode) (*FilterOutput, int) {
	// First try to find the 'first' citation element. For example <co: 1,2,3>
	startFirstCitationStr := startFirstCit
	if f.cmd3Citations {
		startFirstCitationStr = startFirstCitCmd3
	}
	startFirstID, endFirstID, _ := f.findAnElement(str, startFirstCitationStr, endOfCit, f.cmd3Citations)

	// No citation was found so send the plain text and remove from buffer
	if startFirstID < 0 {
		f.curTextIndex += utf8.RuneCountInString(str)
		f.curTextByteIndex += len(str)
		return &FilterOutput{
			Text: str,
		}, len(str)
	}

	//  Only partial citation found so we need to wait for the complete citation. Continue sending nothing.
	if endFirstID < 0 {
		return nil, 0
	}

	// Then try to find the 'last' citation element. For example </co: 1,2,3>
	startLastID, endLastID, docsLast := f.findAnElement(str, startLastCit, endOfCit, f.cmd3Citations)

	//  Only partial citation found so we need to wait for the complete citation.
	if startLastID < 0 || endLastID < 0 {
		if !f.streamNonGroundedAnswer && endLastID == -1 {
			// Send anything before the citation + the inside of the citation
			txt, remove := f.getPartialOrMalformedCitationText(startFirstID, endFirstID, startLastID, str)
			if txt != "" {
				return &FilterOutput{
					Text: txt,
				}, remove
			}
		}
		return nil, 0
	}

	if endFirstID > startLastID {
		// Temp logger to find panic issue.
		f.logger.Warn("Invalid citation",
			zap.String("text", str),
			zap.Int("startFirstID", startFirstID),
			zap.Int("startLastID", startLastID))
		return nil, 0
	}

	// We have found a whole citation, now find the indexes for the citation
	startIndex := f.curTextIndex + startFirstID
	endOfCit := endLastID + 1
	citTxt := str[endFirstID+1 : startLastID]
	text := str[:startFirstID] + citTxt
	f.curTextIndex += utf8.RuneCountInString(text) // Add before checking what has been sent
	f.curTextByteIndex += len(text)
	if f.curCitationByteIndex != -1 {
		// We have already sent some text so just send till the end
		if f.curCitationByteIndex < startLastID {
			text = str[f.curCitationByteIndex:startLastID]
		} else {
			text = "" // Already sent everything
		}
	}
	f.curCitationByteIndex = -1 // reset as we have finished the citation

	cits := []FilterCitation{
		{
			StartIndex: startIndex,
			EndIndex:   startIndex + utf8.RuneCountInString(citTxt),
			Text:       citTxt,
			Sources:    docsLast,
			IsThinking: mode == toolReason,
		}}
	// Recurse to find more partial or complete citations
	moreCits, moreRem := f.ParseCitations(str[endOfCit:], mode)
	if moreCits != nil {
		cits = append(cits, moreCits.Citations...)
		text += moreCits.Text
	}

	return &FilterOutput{
		Text:      text,
		Citations: cits,
	}, endOfCit + moreRem
}

// We want to send the text inside a citation "<co: 4,3>We want to send this</co: 4,3>" with streaming so first "We" then "want"
// Here we get the text in the citation that we haven't already sent. We know this from the curCitationByteIndex
// The curCitationByteIndex marks the index of the citation we have already sent. For example if we have sent "We" the index would be 10
// We also want to send and remove anything before the citation for example "text beforehand<co: 4,3>We want to send this</co: 4,3>"
// We return removing the text before
func (f *filter) getPartialCitationText(startFirstID int, endFirstID int, startLastID int, str string) (string, int) {
	// Send anything before the citation + the inside of the citation
	textBeforeCitation := str[:startFirstID]
	f.curTextIndex += utf8.RuneCountInString(textBeforeCitation)
	f.curTextByteIndex += len(textBeforeCitation)
	startIdx := f.curCitationByteIndex
	if startIdx == -1 {
		startIdx = endFirstID + 1
	}
	f.curCitationByteIndex = len(str) - len(textBeforeCitation)
	endIdx := len(str)
	if startLastID > 0 {
		endIdx = startLastID
	}
	if startIdx >= endIdx {
		return textBeforeCitation, len(textBeforeCitation)
	}
	return textBeforeCitation + str[startIdx:endIdx], len(textBeforeCitation) // Remove only the beginning of the citation
}

func (f *filter) getPartialOrMalformedCitationText(startFirstID, endFirstID, startLastID int, str string) (string, int) {
	// Send anything before the citation + the inside of the citation if the citation is complete and real
	if !f.cmd3Citations || (f.cmd3Citations && len(startFirstCitCmd3)+startFirstID == endFirstID) {
		return f.getPartialCitationText(startFirstID, endFirstID, startLastID, str)
	}
	// otherwise we have a malformed citation, we should send the entire text up to start of the closing citation tag
	// we only do this for cmd3+ models since we dont trust that older models will not hallucinate a malformed citation
	txt := ""
	if startLastID > 0 {
		txt = str[:startLastID]
	} else {
		txt = str
	}
	f.curTextIndex += utf8.RuneCountInString(txt)
	f.curTextByteIndex += len(txt)
	return txt, len(txt)
}

// Given a string this function tries to find the index of the start and end of a citation and list of document indexes in the middle
// Example input str = " <co: 1" and start = "<co: " and end = ">"
// Output:
// 1. index of the start -  if -1 then doesn't exist
// 2. index of the end - if -1 and start > 0 then partial, otherwise whole citation
// 3. the middle of the element
func (f *filter) findAnElement(str string, start string, end string, cmd3Citations bool) (int, int, []Source) {
	startID, startFound := findPartial(str, []string{start})

	// No citation present
	if startID < 0 {
		return -1, -1, nil
	}

	// Partial citation present e.g. '<c"
	if startFound == "" {
		return startID, -1, nil
	}

	// Find the end of the start element citation '>'
	endID := strings.Index(str[startID+1:], end)

	// No '>' so partial citation so continue
	if endID < 0 {
		return startID, -1, nil
	}

	// Now we have both "<co: " and ">" so we have a full element e.g. <co: 1,2,3>
	subString := str[startID+len(start) : startID+1+endID]

	var docIndices []Source
	if cmd3Citations {
		docIndices = f.convertStringToDocIndices(subString)
	} else {
		intIndices := convertStringToIntList(subString)
		if len(intIndices) != 0 {
			docIndices = []Source{{ToolCallIndex: 0, ToolResultIndices: intIndices}} // cmd < 3 always sets tool_index to 0 since it is not returned from the model
		}
	}

	return startID, startID + 1 + endID, docIndices
}

func convertStringToIntList(s string) []int {
	stringIndexes := strings.Split(s, ",")
	// TODO(): Handle errors in filter stream
	intArr := []int{}
	for _, a := range stringIndexes {
		j, err := strconv.Atoi(a)
		if err == nil && j >= 0 {
			intArr = append(intArr, j)
		}
	}
	return intArr
}

func (f *filter) convertStringToDocIndices(s string) []Source {
	stringSplits := strings.Split(strings.TrimSpace(s), "]")
	// TODO(): Handle errors in filter stream
	docIndices := []Source{}
	for _, cit := range stringSplits[:len(stringSplits)-1] {
		citSplits := strings.Split(strings.TrimLeft(cit, ","), ":")
		if len(citSplits) != 2 {
			f.logger.Warn("Invalid citation, not 2 elements after split on ':'", zap.Int("len(citSplits)", len(citSplits)))
			continue
		}

		toolIdxStr, resultIndicesStr := citSplits[0], citSplits[1]
		toolIndex, err := strconv.Atoi(strings.TrimSpace(toolIdxStr))
		if err != nil || toolIndex < 0 {
			f.logger.Warn("Invalid citation", zap.Error(err), zap.Int("toolIndex", toolIndex))
			continue
		}

		resultIndices := []int{}
		resultIdxSplits := strings.Split(strings.TrimLeft(resultIndicesStr, "["), ",")
		for _, resultSplit := range resultIdxSplits {
			resultIdx, err := strconv.Atoi(strings.TrimSpace(resultSplit))
			if err != nil || resultIdx < 0 {
				f.logger.Warn("Invalid citation, could not covert to int", zap.Error(err), zap.Int("resultIdx", resultIdx))
				// Should we just continue here or break out? Right now following existing logic of convertStringToIntList and continuing
				continue
			}
			resultIndices = append(resultIndices, resultIdx)
		}

		docIndex := Source{ToolCallIndex: toolIndex, ToolResultIndices: resultIndices}
		docIndices = append(docIndices, docIndex)
	}
	return docIndices
}
