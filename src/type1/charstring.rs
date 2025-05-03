// use crate::{GlyphId, OutlineBuilder, Rect, RectF};
// use crate::cff::argstack::ArgumentsStack;
// use crate::cff::{Builder, CFFError, IsEven};
// use crate::cff::cff::{operator, MAX_ARGUMENTS_STACK_LEN};
// use crate::cff::charstring::CharStringParser;
// use crate::type1::Parameters;
// use crate::type1::stream::Stream;
//
// struct CharStringParserContext<'a> {
//     params: &'a Parameters,
//     width: Option<f32>,
//     stems_len: u32,
//     has_endchar: bool,
//     has_seac: bool,
//     glyph_id: GlyphId,
// }
//
// fn parse_char_string(
//     data: &[u8],
//     params: &Parameters,
//     glyph_id: GlyphId,
//     width_only: bool,
//     builder: &mut dyn OutlineBuilder,
// ) -> Result<(Rect, Option<f32>), CFFError> {
//     let mut ctx = CharStringParserContext {
//         params,
//         width: None,
//         stems_len: 0,
//         has_endchar: false,
//         has_seac: false,
//         glyph_id,
//     };
//
//     let mut inner_builder = Builder {
//         builder,
//         bbox: RectF::new(),
//     };
//
//     let stack = ArgumentsStack {
//         data: &mut [0.0; MAX_ARGUMENTS_STACK_LEN], // 192B
//         len: 0,
//         max_len: MAX_ARGUMENTS_STACK_LEN,
//     };
//     let mut parser = CharStringParser {
//         stack,
//         builder: &mut inner_builder,
//         x: 0.0,
//         y: 0.0,
//         has_move_to: false,
//         is_first_move_to: true,
//         width_only,
//     };
//     _parse_char_string(&mut ctx, data, 0, &mut parser)?;
//
//     if width_only {
//         return Ok((Rect::zero(), ctx.width));
//     }
//
//     if !ctx.has_endchar {
//         return Err(CFFError::MissingEndChar);
//     }
//
//     let bbox = parser.builder.bbox;
//
//     // Check that bbox was changed.
//     if bbox.is_default() {
//         return Err(CFFError::ZeroBBox);
//     }
//
//     let rect = bbox.to_rect().ok_or(CFFError::BboxOverflow)?;
//     Ok((rect, ctx.width))
// }
//
// fn _parse_char_string(
//     ctx: &mut CharStringParserContext,
//     char_string: &[u8],
//     depth: u8,
//     p: &mut CharStringParser,
// ) -> Result<(), CFFError> {
//     let mut s = Stream::new(char_string);
//     while !s.at_end() {
//         let op = s.read_byte().ok_or(CFFError::ReadOutOfBounds)?;
//         match op {
//             0 | 2 | 9 | 13 | 15 | 16 | 17 => {
//                 // Reserved.
//                 return Err(CFFError::InvalidOperator);
//             }
//             operator::HORIZONTAL_STEM
//             | operator::VERTICAL_STEM
//             | operator::HORIZONTAL_STEM_HINT_MASK
//             | operator::VERTICAL_STEM_HINT_MASK => {
//                 // y dy {dya dyb}* hstem
//                 // x dx {dxa dxb}* vstem
//                 // y dy {dya dyb}* hstemhm
//                 // x dx {dxa dxb}* vstemhm
//
//                 // If the stack length is uneven, than the first value is a `width`.
//                 let len = if p.stack.len().is_odd() && ctx.width.is_none() {
//                     ctx.width = Some(p.stack.at(0));
//                     p.stack.len() - 1
//                 } else {
//                     p.stack.len()
//                 };
//
//                 ctx.stems_len += len as u32 >> 1;
//
//                 // We are ignoring the hint operators.
//                 p.stack.clear();
//             }
//             operator::VERTICAL_MOVE_TO => {
//                 let mut i = 0;
//                 if p.stack.len() == 2 {
//                     i += 1;
//                     if ctx.width.is_none() {
//                         ctx.width = Some(p.stack.at(0));
//                     }
//                 }
//
//                 p.parse_vertical_move_to(i)?;
//             }
//             operator::LINE_TO => {
//                 p.parse_line_to()?;
//             }
//             operator::HORIZONTAL_LINE_TO => {
//                 p.parse_horizontal_line_to()?;
//             }
//             operator::VERTICAL_LINE_TO => {
//                 p.parse_vertical_line_to()?;
//             }
//             operator::CURVE_TO => {
//                 p.parse_curve_to()?;
//             }
//             operator::CALL_LOCAL_SUBROUTINE => {
//                 if p.stack.is_empty() {
//                     return Err(CFFError::InvalidArgumentsStackLength);
//                 }
//
//                 if depth == STACK_LIMIT {
//                     return Err(CFFError::NestingLimitReached);
//                 }
//
//                 // Parse and remember the local subroutine for the current glyph.
//                 // Since it's a pretty complex task, we're doing it only when
//                 // a local subroutine is actually requested by the glyphs charstring.
//                 if ctx.local_subrs.is_none() {
//                     if let FontKind::CID(ref cid) = ctx.metadata.kind {
//                         ctx.local_subrs =
//                             parse_cid_local_subrs(ctx.metadata.table_data, ctx.glyph_id, cid);
//                     }
//                 }
//
//                 if let Some(local_subrs) = ctx.local_subrs {
//                     let subroutine_bias = calc_subroutine_bias(local_subrs.len());
//                     let index = conv_subroutine_index(p.stack.pop(), subroutine_bias)?;
//                     let char_string = local_subrs
//                         .get(index)
//                         .ok_or(CFFError::InvalidSubroutineIndex)?;
//                     _parse_char_string(ctx, char_string, depth + 1, p)?;
//                 } else {
//                     return Err(CFFError::NoLocalSubroutines);
//                 }
//
//                 if ctx.has_endchar && !ctx.has_seac {
//                     if !s.at_end() {
//                         return Err(CFFError::DataAfterEndChar);
//                     }
//
//                     break;
//                 }
//             }
//             operator::RETURN => {
//                 break;
//             }
//             TWO_BYTE_OPERATOR_MARK => {
//                 // flex
//                 let op2 = s.read::<u8>().ok_or(CFFError::ReadOutOfBounds)?;
//                 match op2 {
//                     operator::HFLEX => p.parse_hflex()?,
//                     operator::FLEX => p.parse_flex()?,
//                     operator::HFLEX1 => p.parse_hflex1()?,
//                     operator::FLEX1 => p.parse_flex1()?,
//                     operator::DOTSECTION => {}
//                     _ => return Err(CFFError::UnsupportedOperator),
//                 }
//             }
//             operator::ENDCHAR => {
//                 if p.stack.len() == 4 || (ctx.width.is_none() && p.stack.len() == 5) {
//                     // Process 'seac'.
//                     let accent_char = seac_code_to_glyph_id(&ctx.metadata.charset, p.stack.pop())
//                         .ok_or(CFFError::InvalidSeacCode)?;
//                     let base_char = seac_code_to_glyph_id(&ctx.metadata.charset, p.stack.pop())
//                         .ok_or(CFFError::InvalidSeacCode)?;
//                     let dy = p.stack.pop();
//                     let dx = p.stack.pop();
//
//                     if ctx.width.is_none() && !p.stack.is_empty() {
//                         ctx.width = Some(p.stack.pop())
//                     }
//
//                     ctx.has_seac = true;
//
//                     if depth == STACK_LIMIT {
//                         return Err(CFFError::NestingLimitReached);
//                     }
//
//                     let base_char_string = ctx
//                         .metadata
//                         .char_strings
//                         .get(u32::from(base_char.0))
//                         .ok_or(CFFError::InvalidSeacCode)?;
//                     _parse_char_string(ctx, base_char_string, depth + 1, p)?;
//                     p.x = dx;
//                     p.y = dy;
//
//                     let accent_char_string = ctx
//                         .metadata
//                         .char_strings
//                         .get(u32::from(accent_char.0))
//                         .ok_or(CFFError::InvalidSeacCode)?;
//                     _parse_char_string(ctx, accent_char_string, depth + 1, p)?;
//                 } else if p.stack.len() == 1 && ctx.width.is_none() {
//                     ctx.width = Some(p.stack.pop());
//                 }
//
//                 if !p.is_first_move_to {
//                     p.is_first_move_to = true;
//                     p.builder.close();
//                 }
//
//                 if !s.at_end() {
//                     return Err(CFFError::DataAfterEndChar);
//                 }
//
//                 ctx.has_endchar = true;
//
//                 break;
//             }
//             operator::HINT_MASK | operator::COUNTER_MASK => {
//                 let mut len = p.stack.len();
//
//                 // We are ignoring the hint operators.
//                 p.stack.clear();
//
//                 // If the stack length is uneven, than the first value is a `width`.
//                 if len.is_odd() {
//                     len -= 1;
//                     if ctx.width.is_none() {
//                         ctx.width = Some(p.stack.at(0));
//                     }
//                 }
//
//                 ctx.stems_len += len as u32 >> 1;
//
//                 s.advance(usize::num_from((ctx.stems_len + 7) >> 3));
//             }
//             operator::MOVE_TO => {
//                 let mut i = 0;
//                 if p.stack.len() == 3 {
//                     i += 1;
//                     if ctx.width.is_none() {
//                         ctx.width = Some(p.stack.at(0));
//                     }
//                 }
//
//                 p.parse_move_to(i)?;
//             }
//             operator::HORIZONTAL_MOVE_TO => {
//                 let mut i = 0;
//                 if p.stack.len() == 2 {
//                     i += 1;
//                     if ctx.width.is_none() {
//                         ctx.width = Some(p.stack.at(0));
//                     }
//                 }
//
//                 p.parse_horizontal_move_to(i)?;
//             }
//             operator::CURVE_LINE => {
//                 p.parse_curve_line()?;
//             }
//             operator::LINE_CURVE => {
//                 p.parse_line_curve()?;
//             }
//             operator::VV_CURVE_TO => {
//                 p.parse_vv_curve_to()?;
//             }
//             operator::HH_CURVE_TO => {
//                 p.parse_hh_curve_to()?;
//             }
//             operator::SHORT_INT => {
//                 let n = s.read::<i16>().ok_or(CFFError::ReadOutOfBounds)?;
//                 p.stack.push(f32::from(n))?;
//             }
//             operator::CALL_GLOBAL_SUBROUTINE => {
//                 if p.stack.is_empty() {
//                     return Err(CFFError::InvalidArgumentsStackLength);
//                 }
//
//                 if depth == STACK_LIMIT {
//                     return Err(CFFError::NestingLimitReached);
//                 }
//
//                 let subroutine_bias = calc_subroutine_bias(ctx.metadata.global_subrs.len());
//                 let index = conv_subroutine_index(p.stack.pop(), subroutine_bias)?;
//                 let char_string = ctx
//                     .metadata
//                     .global_subrs
//                     .get(index)
//                     .ok_or(CFFError::InvalidSubroutineIndex)?;
//                 _parse_char_string(ctx, char_string, depth + 1, p)?;
//
//                 if ctx.has_endchar && !ctx.has_seac {
//                     if !s.at_end() {
//                         return Err(CFFError::DataAfterEndChar);
//                     }
//
//                     break;
//                 }
//             }
//             operator::VH_CURVE_TO => {
//                 p.parse_vh_curve_to()?;
//             }
//             operator::HV_CURVE_TO => {
//                 p.parse_hv_curve_to()?;
//             }
//             32..=246 => {
//                 p.parse_int1(op)?;
//             }
//             247..=250 => {
//                 p.parse_int2(op, &mut s)?;
//             }
//             251..=254 => {
//                 p.parse_int3(op, &mut s)?;
//             }
//             operator::FIXED_16_16 => {
//                 p.parse_fixed(&mut s)?;
//             }
//         }
//
//         if p.width_only && ctx.width.is_some() {
//             break;
//         }
//     }
//
//     // TODO: 'A charstring subroutine must end with either an endchar or a return operator.'
//
//     Ok(())
// }
//
// fn seac_code_to_glyph_id(charset: &Charset, n: f32) -> Option<GlyphId> {
//     let code = u8::try_num_from(n)?;
//
//     let sid = STANDARD_ENCODING[usize::from(code)];
//     let sid = StringId(u16::from(sid));
//
//     match charset {
//         Charset::ISOAdobe => {
//             // ISO Adobe charset only defines string ids up to 228 (zcaron)
//             if code <= 228 {
//                 Some(GlyphId(sid.0))
//             } else {
//                 None
//             }
//         }
//         Charset::Expert | Charset::ExpertSubset => None,
//         _ => charset.sid_to_gid(sid),
//     }
// }
