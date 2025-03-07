//
//  People.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 6/3/25.
//

import SwiftUI
import Foundation

struct PeopleShareView: View {
    var body: some View {
        VStack {
            Button(action: {}) {
                HStack {
                    Text("Find or add new people here")
                        .modifier(Label1())
                        .foregroundStyle(Theme.PrimaryText.color)
                    Spacer()
                }
            }
            .padding(.vertical, 12)
            .padding(.horizontal, 20)
            .frame(maxWidth: .infinity)
            .background(Theme.PrimaryText.color.opacity(0.15))
            .clipShape(Capsule())
        }
    }
}
