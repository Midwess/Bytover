//
//  Text.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct Body1: ViewModifier {
    func body(content: Content) -> some View {
        content.lineSpacing(8)
            .font(FontTheme.Body1)
    }
}

struct Body2: ViewModifier {
    func body(content: Content) -> some View {
        content.lineSpacing(8)
            .font(FontTheme.Body2)
    }
}

struct H1Heading: ViewModifier {
    func body(content: Content) -> some View {
        content
            .lineSpacing(12)
            .font(FontTheme.H1Heading)
    }
}

struct Label1: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(FontTheme.Label1)
            .fontWeight(.bold)
    }
}

struct Label4: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(FontTheme.Label4)
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

