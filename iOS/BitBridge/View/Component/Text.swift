//
//  Text.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct Heading1: ViewModifier {
    func body(content: Content) -> some View {
        content
            .lineSpacing(12)
            .font(FontTheme.H1Heading)
            .foregroundColor(Theme.LightViolet.color)
    }
}

struct Heading2: ViewModifier {
    func body(content: Content) -> some View {
        content
            .lineSpacing(12)
            .font(FontTheme.H2Heading)
            .foregroundColor(Theme.LightViolet.color)
    }
}

struct Body1: ViewModifier {
    func body(content: Content) -> some View {
        content.lineSpacing(8)
            .font(FontTheme.Body1)
            .foregroundColor(Theme.LightViolet.color)
    }
}

struct Body2: ViewModifier {
    func body(content: Content) -> some View {
        content.lineSpacing(8)
            .font(FontTheme.Body2)
            .foregroundColor(Theme.LightViolet.color)
    }
}

struct H1Heading: ViewModifier {
    func body(content: Content) -> some View {
        content
            .lineSpacing(12)
            .font(FontTheme.H1Heading)
            .foregroundColor(Theme.LightViolet.color)
    }
}

struct Label1: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(FontTheme.Label1)
            .fontWeight(.medium)
            .foregroundColor(Theme.LightViolet.color)
    }
}

struct Label2: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(FontTheme.Label2)
            .foregroundColor(Theme.LightViolet.color)
    }
}

struct Label3: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(FontTheme.Label3)
            .foregroundColor(Theme.LightViolet.color.opacity(0.7))
    }
}

struct Label4: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(FontTheme.Label4)
            .foregroundColor(Theme.LightViolet.color)
    }
}

struct GradientHeading: ViewModifier {
    func body(content: Content) -> some View {
        content
            .modifier(H1Heading())
            .foregroundStyle(Theme.gradientHeading)
            .multilineTextAlignment(.leading)
    }
}

struct GradientHeadingView: View {
    var body: some View {
        Text("Demo text ah dj dajw dj tn dawj dawjnt jn ")
            .frame(maxWidth: .infinity)
            .font(FontTheme.H1Heading)
            .modifier(GradientHeading())
    }
}

struct GradientHeadingView_Preview: PreviewProvider {
    static var previews: some View {
        VStack {
            Spacer()
            GradientHeadingView()
                .padding()
            Spacer()
        }
        .frame(width: .infinity, height: .infinity)
        .background(Theme.DarkViolet.color)
    }
}

